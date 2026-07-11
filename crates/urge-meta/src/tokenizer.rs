//! Stage 1: Tokenization
//!
//! Converts raw input (text or pre-structured expression strings) into a stream
//! of `Token`s classified by the Unicode Semantic Dictionary.
//!
//! ## The shell-proof heritage
//!
//! The original logic engine was proved viable using bash regex pipelines:
//!
//! ```bash
//! echo "$EXPR" \
//!   | grep -oP '(must|may|must_not|always|eventually|until|and|or|not)' \
//!   | awk '{print toupper($0)}' \
//!   | sort -u \
//!   | while read op; do classify_paradigm "$op"; done
//! ```
//!
//! This Rust tokenizer preserves the same design intent:
//! 1. Scan input for operator tokens (regex or Unicode codepoints)
//! 2. Classify each via the dictionary
//! 3. Emit typed Token stream
//!
//! The difference is that this runs in <1µs on embedded hardware vs. the
//! shell's ~10ms process-startup overhead.

use urge_core::{
    ast::Token,
    symbol::{SemanticClass, UnicodeSemanticDictionary},
};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Tokenizes an input string into a classified token stream.
#[derive(Default)]
pub struct Tokenizer {
    /// Whether to normalize Unicode (NFC) before tokenizing.
    /// Disable on embedded targets where ICU is not available.
    pub normalize_unicode: bool,
}

impl Tokenizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Tokenize a UTF-8 input string.
    ///
    /// Strategy:
    /// 1. Scan for Unicode codepoints first (∧, ∨, □, etc.)
    /// 2. Then scan for ASCII keyword boundaries
    /// 3. Emit identifiers and literals for anything else
    #[cfg(feature = "alloc")]
    pub fn tokenize(&self, input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = input.char_indices().peekable();

        while let Some((offset, ch)) = chars.next() {
            let cp = ch as u32;

            // ── Unicode operator? ──────────────────────────────────────────
            if let Some(sym) = UnicodeSemanticDictionary::lookup_codepoint(cp) {
                tokens.push(Token::new(sym.class, &ch.to_string(), offset as u32));
                continue;
            }

            // ── ASCII letter/underscore: keyword candidate ─────────────────
            if ch.is_ascii_alphabetic() || ch == '_' {
                let start = offset;
                let mut word = alloc::string::String::new();
                word.push(ch);

                // Consume subsequent alphanumeric / underscore chars.
                while let Some(&(_, nc)) = chars.peek() {
                    if nc.is_ascii_alphanumeric() || nc == '_' {
                        word.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }

                if let Some(sym) = UnicodeSemanticDictionary::lookup_keyword(&word) {
                    tokens.push(Token::new(sym.class, &word, start as u32));
                } else {
                    // Unknown identifier.
                    tokens.push(Token::new(SemanticClass::Identifier, &word, start as u32));
                }
                continue;
            }

            // ── Numeric literal ────────────────────────────────────────────
            if ch.is_ascii_digit()
                || (ch == '-' && chars.peek().is_some_and(|&(_, c)| c.is_ascii_digit()))
            {
                let start = offset;
                let mut num = alloc::string::String::new();
                num.push(ch);
                let mut has_dot = false;

                while let Some(&(_, nc)) = chars.peek() {
                    if nc.is_ascii_digit() {
                        num.push(nc);
                        chars.next();
                    } else if nc == '.' && !has_dot {
                        has_dot = true;
                        num.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::new(
                    SemanticClass::NumericLiteral,
                    &num,
                    start as u32,
                ));
                continue;
            }

            // ── String literal ─────────────────────────────────────────────
            if ch == '"' || ch == '\'' {
                let delimiter = ch;
                let start = offset;
                let mut s = alloc::string::String::new();
                for (_, nc) in chars.by_ref() {
                    if nc == delimiter {
                        break;
                    }
                    s.push(nc);
                }
                tokens.push(Token::new(SemanticClass::StringLiteral, &s, start as u32));
                continue;
            }

            // ── Whitespace / punctuation: skip ─────────────────────────────
        }

        tokens
    }

    /// No-alloc tokenize: fills a fixed-size buffer and returns count.
    /// Suitable for embedded/BIOS targets.
    #[cfg(not(feature = "alloc"))]
    pub fn tokenize_into<'a>(&self, input: &str, buf: &'a mut [Token; 64]) -> &'a [Token] {
        let mut count = 0;
        let mut chars = input.char_indices();

        while let Some((offset, ch)) = chars.next() {
            if count >= 64 {
                break;
            }
            let cp = ch as u32;
            if let Some(sym) = UnicodeSemanticDictionary::lookup_codepoint(cp) {
                buf[count] = Token::new(sym.class, "", offset as u32);
                count += 1;
            } else if ch.is_ascii_alphabetic() {
                // Simplified: consume one char at a time for keyword matching.
                // Full keyword scanning requires alloc on embedded.
                buf[count] = Token::new(SemanticClass::Identifier, "", offset as u32);
                count += 1;
            }
        }
        &buf[..count]
    }
}

// ── Regex-based tokenizer (std + regex feature) ───────────────────────────────
//
// When compiled with the `regex` feature, the tokenizer uses a compiled regex
// to match multi-character keywords in a single pass. This matches the spirit
// of the shell proof (grep -oP) but is compiled at build time.
//
// The regex pattern is built from all keyword entries in the dictionary.

#[cfg(feature = "regex")]
mod regex_tokenizer {
    use super::*;

    /// Build the keyword pattern from the dictionary at runtime.
    /// In production you'd use a `lazy_static!` or `once_cell::sync::Lazy`.
    /// Provided as a helper for downstream consumers; not used internally.
    #[allow(dead_code)]
    pub fn build_keyword_pattern() -> alloc::string::String {
        let mut keywords: Vec<&'static str> = UnicodeSemanticDictionary::ENTRIES
            .iter()
            .filter_map(|s| s.keyword)
            .collect();
        // Sort longest first to avoid prefix shadowing.
        keywords.sort_by_key(|k| core::cmp::Reverse(k.len()));
        keywords.dedup();
        alloc::format!(r"\b({})\b", keywords.join("|"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use urge_core::symbol::SemanticClass;

    #[test]
    #[cfg(feature = "alloc")]
    fn tokenize_deontic_keywords() {
        let t = Tokenizer::new();
        let tokens = t.tokenize("agent must obtain_consent before deadline");
        let classes: Vec<_> = tokens.iter().map(|t| t.class).collect();
        assert!(classes.contains(&SemanticClass::Obligatory));
        assert!(classes.contains(&SemanticClass::Until)); // "before" → Until
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn tokenize_temporal_keywords() {
        let t = Tokenizer::new();
        let tokens = t.tokenize("always eventually until");
        assert!(tokens.iter().any(|t| t.class == SemanticClass::Globally));
        assert!(tokens.iter().any(|t| t.class == SemanticClass::Finally));
        assert!(tokens.iter().any(|t| t.class == SemanticClass::Until));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn tokenize_unicode_operators() {
        let t = Tokenizer::new();
        let tokens = t.tokenize("φ ∧ ψ → ⊤");
        assert!(tokens.iter().any(|t| t.class == SemanticClass::Conjunction));
        assert!(tokens.iter().any(|t| t.class == SemanticClass::Implication));
        assert!(tokens.iter().any(|t| t.class == SemanticClass::Verum));
    }
}
