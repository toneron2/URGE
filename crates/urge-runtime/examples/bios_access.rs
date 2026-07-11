//! BIOS-level access control governance.
//!
//! Demonstrates the "governance chip" concept from the patent: a tiny,
//! deterministic core that governs device capability access with no
//! inference, no heap allocation in the hot path, and full auditability.
//!
//! This is the embedded / BIOS deployment target. The same logic engine
//! that runs in a healthcare ERP can govern a microcontroller.

use urge_runtime::embedded::BiosGovernor;

fn main() {
    println!("=== URGE BIOS Access Control Governance Demo ===\n");

    let gov = BiosGovernor::new();

    struct Request {
        capability: &'static str,
        app: &'static str,
        battery: u8,
        expected: bool,
    }

    let requests = &[
        Request {
            capability: "camera",
            app: "app.photos",
            battery: 80,
            expected: true,
        },
        Request {
            capability: "camera",
            app: "app.photos",
            battery: 3,
            expected: false,
        }, // critical battery
        Request {
            capability: "gps",
            app: "app.maps",
            battery: 25,
            expected: true,
        },
        Request {
            capability: "gps",
            app: "app.maps",
            battery: 2,
            expected: false,
        }, // critical battery
        Request {
            capability: "microphone",
            app: "app.voice",
            battery: 50,
            expected: true,
        },
        Request {
            capability: "nfc",
            app: "app.pay",
            battery: 15,
            expected: false,
        }, // below 20% threshold
        Request {
            capability: "wifi",
            app: "app.browser",
            battery: 90,
            expected: true,
        },
    ];

    println!(
        "{:<15} {:<15} {:<10} {:<10} {:<8}",
        "Capability", "App", "Battery%", "Permitted", "Correct?"
    );
    println!("{}", "-".repeat(63));

    for req in requests {
        let permitted = gov.check_access(req.capability, req.app, req.battery);
        let correct = permitted == req.expected;
        println!(
            "{:<15} {:<15} {:<10} {:<10} {:<8}",
            req.capability,
            req.app,
            format!("{}%", req.battery),
            if permitted { "YES ✓" } else { "NO  ✗" },
            if correct { "✓" } else { "FAIL" },
        );
    }

    println!("\n=== Key properties demonstrated ===");
    println!("  • Deterministic: same inputs → same output, always");
    println!("  • Auditability: every decision has a logic trace");
    println!("  • No inference: pure formal logic, no learned weights");
    println!("  • Embedded-capable: no heap allocation in access-control path");
    println!("  • <1µs decision time on ARM Cortex-M4 (est.)");
    println!("\nThis is the 'BIOS governance chip' concept:");
    println!("  The same engine governs a medical device AND a healthcare ERP.");
    println!("  Same paradigms. Same Unicode dictionary. Same audit trail format.");
    println!("  Different policies. Universal governance layer.");
}
