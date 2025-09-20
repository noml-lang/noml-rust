#![allow(clippy::uninlined_format_args)]

// TOML vs NOML Feature Comparison Analysis
// Focusing on READ-TIME functionality only

fn main() {
    println!("=== TOML vs NOML: Read-Time Feature Analysis ===\n");

    // Core TOML features that both support
    let toml_features = vec![
        "String values",
        "Integer values",
        "Float values",
        "Boolean values",
        "Arrays",
        "Tables (objects)",
        "Nested tables",
        "Array of tables",
        "Comments (preserved in AST)",
        "Unicode support",
        "Multi-line strings",
        "Literal strings",
        "Basic key-value access",
    ];

    // Additional NOML read-time features
    let noml_exclusive_features = vec![
        // Path-based access
        "Dot-notation path access (\"server.database.port\")",
        "Deep nesting without section syntax",
        // Native type system
        "Native size parsing (@size(\"10MB\") -> bytes)",
        "Native duration parsing (@duration(\"30s\") -> seconds)",
        "Native URL parsing (@url(\"https://...\") -> URL object)",
        "Native binary data (@base64(\"...\"))",
        // Dynamic resolution (at read-time)
        "Environment variable resolution (env(\"VAR\", \"default\"))",
        "String interpolation (\"Hello ${name}!\")",
        "Include file composition (include \"other.noml\")",
        // Advanced value access
        "Type coercion (string \"123\" -> integer 123)",
        "Boolean parsing (\"yes\"/\"true\"/\"1\" -> true)",
        "Flexible number parsing (\"3.14\" as int/float)",
        "Optional value access with defaults",
        // Source preservation for tooling
        "Exact source location tracking",
        "Format preservation (spacing, comments)",
        "Round-trip editing capability",
        // Error handling
        "Rich error messages with source context",
        "Path-based error reporting",
        "Type mismatch diagnostics",
    ];

    println!("📊 FEATURE INVENTORY");
    println!("TOML baseline features: {}", toml_features.len());
    println!("NOML exclusive features: {}", noml_exclusive_features.len());
    println!(
        "Total NOML features: {}",
        toml_features.len() + noml_exclusive_features.len()
    );

    let functionality_increase =
        (noml_exclusive_features.len() as f64 / toml_features.len() as f64) * 100.0;
    let total_advantage = ((toml_features.len() + noml_exclusive_features.len()) as f64
        / toml_features.len() as f64)
        * 100.0
        - 100.0;

    println!("\n📈 FUNCTIONALITY ANALYSIS");
    println!(
        "NOML adds {:.0}% more read-time functionality than TOML baseline",
        functionality_increase
    );
    println!(
        "NOML has {:.0}% more total read features than TOML",
        total_advantage
    );

    println!("\n🏆 FEATURE CATEGORIES");

    println!("\n✅ TOML Baseline Features (both support):");
    for (i, feature) in toml_features.iter().enumerate() {
        println!("  {}: {}", i + 1, feature);
    }

    println!("\n🚀 NOML Exclusive Read Features:");
    for (i, feature) in noml_exclusive_features.iter().enumerate() {
        println!("  {}: {}", i + 1, feature);
    }

    // Performance vs functionality trade-off analysis
    println!("\n📊 PERFORMANCE vs FUNCTIONALITY TRADE-OFF");
    println!("Performance cost: 2x slower than TOML");
    println!("Functionality gain: {:.0}% more features", total_advantage);
    println!(
        "Value ratio: {:.1}% functionality per 1% performance cost",
        total_advantage / 100.0
    );

    println!("\n🎯 POUND-FOR-POUND ASSESSMENT");
    println!("TOML: Fast, simple key-value with basic nesting");
    println!(
        "NOML: 2x cost for {}x functionality ({}% more features)",
        (total_advantage / 100.0) + 1.0,
        total_advantage
    );

    // Break down by impact
    println!("\n💥 HIGH-IMPACT EXCLUSIVE FEATURES");
    let high_impact = vec![
        "Path-based access (eliminates navigation boilerplate)",
        "Environment variables (eliminates manual getenv())",
        "Native types (eliminates custom parsing)",
        "String interpolation (eliminates manual templating)",
    ];

    for feature in high_impact {
        println!("  • {}", feature);
    }

    println!("\n🤔 THE VERDICT");
    println!("TOML: 13 core features, blazing fast");
    println!(
        "NOML: {} total features, 2x slower",
        toml_features.len() + noml_exclusive_features.len()
    );
    println!(
        "Trade-off: Pay 100% performance cost for {:.0}% more functionality",
        total_advantage
    );
    println!(
        "That's {:.1} features per performance point!",
        total_advantage / 100.0
    );
}
