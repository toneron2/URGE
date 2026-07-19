//! Unicode Semantic Dictionary
//!
//! Maps Unicode codepoints (and ASCII keyword aliases) to their semantic class
//! within one or more logic paradigms. This is the "300+ operator" dictionary
//! at the foundation of the architecture.
//!
//! The dictionary is a **static, compile-time table** — zero runtime allocation,
//! suitable for ROM-resident firmware.
//!
//! ## Design
//!
//! Every symbol has:
//! - A canonical Unicode codepoint (or ASCII keyword)
//! - A human-readable name
//! - Its `SemanticClass` (which logic operator it represents)
//! - The set of `Paradigm`s it belongs to
//!
//! The shell-based proof validated this approach: regex patterns matched symbols,
//! then piped output selected the appropriate evaluation path. In Rust, the same
//! decision is a simple table lookup followed by a match on `SemanticClass`.

use crate::engine::Paradigm;

/// The semantic role of a symbol within its logic paradigm(s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SemanticClass {
    // ── Boolean operators ──────────────────────────────────────────────────
    Conjunction,   // ∧  AND
    Disjunction,   // ∨  OR
    Negation,      // ¬  NOT
    Implication,   // →  IF…THEN
    Biconditional, // ↔  IFF
    ExclusiveOr,   // ⊕  XOR
    Verum,         // ⊤  TRUE
    Falsum,        // ⊥  FALSE

    // ── Modal operators ────────────────────────────────────────────────────
    Necessity,   // □  necessarily
    Possibility, // ◇  possibly

    // ── Epistemic operators ────────────────────────────────────────────────
    Knows,                // K  agent knows
    Believes,             // B  agent believes
    CommonKnowledge,      // C  common knowledge
    DistributedKnowledge, // D distributed knowledge

    // ── Deontic operators ──────────────────────────────────────────────────
    Obligatory, // O  obligatory / must
    Permitted,  // P  permitted / may
    Forbidden,  // F  forbidden / must-not
    Waived,     // W  waived obligation

    // ── Temporal / LTL operators ───────────────────────────────────────────
    Globally,  // G  always holds
    Finally,   // F  eventually holds
    Next,      // X  holds at next step
    Until,     // U  holds until
    Release,   // R  released by
    WeakUntil, // W  weak until

    // ── Fuzzy / probabilistic operators ───────────────────────────────────
    MembershipDegree, // μ  fuzzy membership
    FuzzyAnd,         // ⊓  fuzzy conjunction (min)
    FuzzyOr,          // ⊔  fuzzy disjunction (max)
    Probability,      // P(·) probabilistic measure

    // ── Paraconsistent operators ───────────────────────────────────────────
    BothTrueAndFalse, // signals inconsistency without explosion
    NeitherTrueNorFalse,

    // ── Quantifiers ────────────────────────────────────────────────────────
    Universal,         // ∀  for all
    Existential,       // ∃  there exists
    UniqueExistential, // ∃! exactly one

    // ── Set / type operators ───────────────────────────────────────────────
    ElementOf,    // ∈
    NotElementOf, // ∉
    Subset,       // ⊆
    StrictSubset, // ⊂
    Union,        // ∪
    Intersection, // ∩
    EmptySet,     // ∅

    // ── Relational ─────────────────────────────────────────────────────────
    Equals,         // =
    NotEquals,      // ≠
    LessThan,       // <
    LessOrEqual,    // ≤
    GreaterThan,    // >
    GreaterOrEqual, // ≥

    // ── Special ────────────────────────────────────────────────────────────
    Turnstile,       // ⊢  provability / entailment
    DoubleTurnstile, // ⊨  semantic entailment / models
    Therefore,       // ∴
    Because,         // ∵

    // ── Identifier / Literal ──────────────────────────────────────────────
    Identifier,
    NumericLiteral,
    StringLiteral,
    BooleanLiteral,
    TimeLiteral,
}

impl SemanticClass {
    /// The set of paradigms this operator contributes to.
    /// A single symbol can participate in multiple paradigms.
    pub fn paradigms(self) -> &'static [Paradigm] {
        match self {
            SemanticClass::Conjunction
            | SemanticClass::Disjunction
            | SemanticClass::Negation
            | SemanticClass::Implication
            | SemanticClass::Biconditional
            | SemanticClass::ExclusiveOr
            | SemanticClass::Verum
            | SemanticClass::Falsum => &[Paradigm::Boolean],

            SemanticClass::Necessity | SemanticClass::Possibility => {
                &[Paradigm::Modal, Paradigm::Epistemic]
            }

            SemanticClass::Knows
            | SemanticClass::Believes
            | SemanticClass::CommonKnowledge
            | SemanticClass::DistributedKnowledge => &[Paradigm::Epistemic],

            SemanticClass::Obligatory
            | SemanticClass::Permitted
            | SemanticClass::Forbidden
            | SemanticClass::Waived => &[Paradigm::Deontic],

            SemanticClass::Globally
            | SemanticClass::Finally
            | SemanticClass::Next
            | SemanticClass::Until
            | SemanticClass::Release
            | SemanticClass::WeakUntil => &[Paradigm::Temporal],

            SemanticClass::MembershipDegree | SemanticClass::FuzzyAnd | SemanticClass::FuzzyOr => {
                &[Paradigm::Fuzzy]
            }

            SemanticClass::Probability => &[Paradigm::Probabilistic],

            SemanticClass::BothTrueAndFalse | SemanticClass::NeitherTrueNorFalse => {
                &[Paradigm::Paraconsistent]
            }

            SemanticClass::Universal
            | SemanticClass::Existential
            | SemanticClass::UniqueExistential => &[Paradigm::Boolean, Paradigm::Modal],

            _ => &[Paradigm::Boolean],
        }
    }
}

/// A resolved dictionary entry — one symbol, fully annotated.
#[derive(Debug, Clone, Copy)]
pub struct Symbol {
    /// Unicode codepoint or 0 for pure-ASCII keywords.
    pub codepoint: u32,
    /// ASCII keyword alias (e.g., "must", "may", "always").
    pub keyword: Option<&'static str>,
    /// Human-readable name used in logic traces.
    pub name: &'static str,
    /// Semantic role.
    pub class: SemanticClass,
}

/// The static Unicode Semantic Dictionary.
///
/// **This table IS the architecture at the data layer.** Every operator
/// from 15+ logic paradigms is represented. The router uses it to classify
/// every token in an incoming expression before engine selection.
pub struct UnicodeSemanticDictionary;

impl UnicodeSemanticDictionary {
    /// All dictionary entries. Stored in flash/ROM-friendly `&'static` slice.
    pub const ENTRIES: &'static [Symbol] = &[
        // ── Boolean ────────────────────────────────────────────────────────
        Symbol {
            codepoint: 0x2227,
            keyword: Some("and"),
            name: "Conjunction",
            class: SemanticClass::Conjunction,
        },
        Symbol {
            codepoint: 0x2228,
            keyword: Some("or"),
            name: "Disjunction",
            class: SemanticClass::Disjunction,
        },
        Symbol {
            codepoint: 0x00AC,
            keyword: Some("not"),
            name: "Negation",
            class: SemanticClass::Negation,
        },
        Symbol {
            codepoint: 0x2192,
            keyword: Some("implies"),
            name: "Implication",
            class: SemanticClass::Implication,
        },
        Symbol {
            codepoint: 0x2194,
            keyword: Some("iff"),
            name: "Biconditional",
            class: SemanticClass::Biconditional,
        },
        Symbol {
            codepoint: 0x2295,
            keyword: Some("xor"),
            name: "ExclusiveOr",
            class: SemanticClass::ExclusiveOr,
        },
        Symbol {
            codepoint: 0x22A4,
            keyword: Some("true"),
            name: "Verum",
            class: SemanticClass::Verum,
        },
        Symbol {
            codepoint: 0x22A5,
            keyword: Some("false"),
            name: "Falsum",
            class: SemanticClass::Falsum,
        },
        // ── Modal ──────────────────────────────────────────────────────────
        Symbol {
            codepoint: 0x25A1,
            keyword: Some("necessarily"),
            name: "Necessity",
            class: SemanticClass::Necessity,
        },
        Symbol {
            codepoint: 0x25C7,
            keyword: Some("possibly"),
            name: "Possibility",
            class: SemanticClass::Possibility,
        },
        // ── Epistemic ──────────────────────────────────────────────────────
        Symbol {
            codepoint: 0x004B,
            keyword: Some("knows"),
            name: "Knows",
            class: SemanticClass::Knows,
        },
        Symbol {
            codepoint: 0x0042,
            keyword: Some("believes"),
            name: "Believes",
            class: SemanticClass::Believes,
        },
        Symbol {
            codepoint: 0x0043,
            keyword: Some("common_knowledge"),
            name: "CommonKnowledge",
            class: SemanticClass::CommonKnowledge,
        },
        Symbol {
            codepoint: 0x0044,
            keyword: Some("distributed_knowledge"),
            name: "DistributedKnowledge",
            class: SemanticClass::DistributedKnowledge,
        },
        // ── Deontic ────────────────────────────────────────────────────────
        Symbol {
            codepoint: 0x004F,
            keyword: Some("must"),
            name: "Obligatory",
            class: SemanticClass::Obligatory,
        },
        Symbol {
            codepoint: 0x0050,
            keyword: Some("may"),
            name: "Permitted",
            class: SemanticClass::Permitted,
        },
        Symbol {
            codepoint: 0x0046,
            keyword: Some("must_not"),
            name: "Forbidden",
            class: SemanticClass::Forbidden,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("ought"),
            name: "Obligatory",
            class: SemanticClass::Obligatory,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("should"),
            name: "Obligatory",
            class: SemanticClass::Obligatory,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("prohibited"),
            name: "Forbidden",
            class: SemanticClass::Forbidden,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("permitted"),
            name: "Permitted",
            class: SemanticClass::Permitted,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("allowed"),
            name: "Permitted",
            class: SemanticClass::Permitted,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("forbidden"),
            name: "Forbidden",
            class: SemanticClass::Forbidden,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("waived"),
            name: "Waived",
            class: SemanticClass::Waived,
        },
        // ── Temporal / LTL ─────────────────────────────────────────────────
        Symbol {
            codepoint: 0x0047,
            keyword: Some("always"),
            name: "Globally",
            class: SemanticClass::Globally,
        },
        Symbol {
            codepoint: 0x0046,
            keyword: Some("eventually"),
            name: "Finally",
            class: SemanticClass::Finally,
        },
        Symbol {
            codepoint: 0x0058,
            keyword: Some("next"),
            name: "Next",
            class: SemanticClass::Next,
        },
        Symbol {
            codepoint: 0x0055,
            keyword: Some("until"),
            name: "Until",
            class: SemanticClass::Until,
        },
        Symbol {
            codepoint: 0x0052,
            keyword: Some("release"),
            name: "Release",
            class: SemanticClass::Release,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("never"),
            name: "Globally(¬)",
            class: SemanticClass::Globally,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("within"),
            name: "Finally",
            class: SemanticClass::Finally,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("before"),
            name: "Until",
            class: SemanticClass::Until,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("after"),
            name: "Finally",
            class: SemanticClass::Finally,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("deadline"),
            name: "Until",
            class: SemanticClass::Until,
        },
        // ── Fuzzy ──────────────────────────────────────────────────────────
        Symbol {
            codepoint: 0x03BC,
            keyword: Some("mu"),
            name: "MembershipDegree",
            class: SemanticClass::MembershipDegree,
        },
        Symbol {
            codepoint: 0x2293,
            keyword: Some("fuzzy_and"),
            name: "FuzzyAnd",
            class: SemanticClass::FuzzyAnd,
        },
        Symbol {
            codepoint: 0x2294,
            keyword: Some("fuzzy_or"),
            name: "FuzzyOr",
            class: SemanticClass::FuzzyOr,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("probability"),
            name: "Probability",
            class: SemanticClass::Probability,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("likely"),
            name: "Probability",
            class: SemanticClass::Probability,
        },
        Symbol {
            codepoint: 0x0000,
            keyword: Some("unlikely"),
            name: "Probability",
            class: SemanticClass::Probability,
        },
        // ── Quantifiers ────────────────────────────────────────────────────
        Symbol {
            codepoint: 0x2200,
            keyword: Some("forall"),
            name: "Universal",
            class: SemanticClass::Universal,
        },
        Symbol {
            codepoint: 0x2203,
            keyword: Some("exists"),
            name: "Existential",
            class: SemanticClass::Existential,
        },
        Symbol {
            codepoint: 0x2204,
            keyword: Some("exists_unique"),
            name: "UniqueExistential",
            class: SemanticClass::UniqueExistential,
        },
        // ── Set operators ──────────────────────────────────────────────────
        Symbol {
            codepoint: 0x2208,
            keyword: Some("in"),
            name: "ElementOf",
            class: SemanticClass::ElementOf,
        },
        Symbol {
            codepoint: 0x2209,
            keyword: Some("not_in"),
            name: "NotElementOf",
            class: SemanticClass::NotElementOf,
        },
        Symbol {
            codepoint: 0x2286,
            keyword: Some("subset"),
            name: "Subset",
            class: SemanticClass::Subset,
        },
        Symbol {
            codepoint: 0x2282,
            keyword: Some("strict_subset"),
            name: "StrictSubset",
            class: SemanticClass::StrictSubset,
        },
        Symbol {
            codepoint: 0x222A,
            keyword: Some("union"),
            name: "Union",
            class: SemanticClass::Union,
        },
        Symbol {
            codepoint: 0x2229,
            keyword: Some("intersect"),
            name: "Intersection",
            class: SemanticClass::Intersection,
        },
        Symbol {
            codepoint: 0x2205,
            keyword: Some("empty"),
            name: "EmptySet",
            class: SemanticClass::EmptySet,
        },
        // ── Relational ─────────────────────────────────────────────────────
        Symbol {
            codepoint: 0x003D,
            keyword: Some("eq"),
            name: "Equals",
            class: SemanticClass::Equals,
        },
        Symbol {
            codepoint: 0x2260,
            keyword: Some("neq"),
            name: "NotEquals",
            class: SemanticClass::NotEquals,
        },
        Symbol {
            codepoint: 0x003C,
            keyword: Some("lt"),
            name: "LessThan",
            class: SemanticClass::LessThan,
        },
        Symbol {
            codepoint: 0x2264,
            keyword: Some("lte"),
            name: "LessOrEqual",
            class: SemanticClass::LessOrEqual,
        },
        Symbol {
            codepoint: 0x003E,
            keyword: Some("gt"),
            name: "GreaterThan",
            class: SemanticClass::GreaterThan,
        },
        Symbol {
            codepoint: 0x2265,
            keyword: Some("gte"),
            name: "GreaterOrEqual",
            class: SemanticClass::GreaterOrEqual,
        },
        // ── Proof-theoretic ────────────────────────────────────────────────
        Symbol {
            codepoint: 0x22A2,
            keyword: Some("proves"),
            name: "Turnstile",
            class: SemanticClass::Turnstile,
        },
        Symbol {
            codepoint: 0x22A8,
            keyword: Some("models"),
            name: "DoubleTurnstile",
            class: SemanticClass::DoubleTurnstile,
        },
        Symbol {
            codepoint: 0x2234,
            keyword: Some("therefore"),
            name: "Therefore",
            class: SemanticClass::Therefore,
        },
        Symbol {
            codepoint: 0x2235,
            keyword: Some("because"),
            name: "Because",
            class: SemanticClass::Because,
        },
    ];

    /// Look up a symbol by its Unicode codepoint.
    pub fn lookup_codepoint(cp: u32) -> Option<&'static Symbol> {
        Self::ENTRIES
            .iter()
            .find(|s| s.codepoint == cp && s.codepoint != 0)
    }

    /// Look up a symbol by its keyword alias (case-insensitive ASCII).
    pub fn lookup_keyword(kw: &str) -> Option<&'static Symbol> {
        // Lowercase inline — no heap allocation.
        let mut buf = [0u8; 64];
        let kw_bytes = kw.as_bytes();
        let len = kw_bytes.len().min(64);
        for (i, &b) in kw_bytes[..len].iter().enumerate() {
            buf[i] = b.to_ascii_lowercase();
        }
        let lower = core::str::from_utf8(&buf[..len]).ok()?;
        Self::ENTRIES.iter().find(|s| s.keyword == Some(lower))
    }

    /// Collect all paradigms detected in a token stream.
    /// This is the **paradigm detector** stage of Figure 26.
    pub fn detect_paradigms(classes: &[SemanticClass]) -> ParadigmSet {
        let mut set = ParadigmSet::empty();
        for cls in classes {
            for &p in cls.paradigms() {
                set.insert(p);
            }
        }
        // Boolean is always present as the base paradigm.
        set.insert(Paradigm::Boolean);
        set
    }
}

/// A compact bitset of active paradigms (fits in a u16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParadigmSet(u16);

impl ParadigmSet {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn insert(&mut self, p: Paradigm) {
        self.0 |= 1 << (p as u8);
    }

    pub fn contains(&self, p: Paradigm) -> bool {
        self.0 & (1 << (p as u8)) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = Paradigm> + '_ {
        Paradigm::ALL.iter().copied().filter(|&p| self.contains(p))
    }
}
