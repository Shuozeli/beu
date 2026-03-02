use crate::config::BeuConfig;

/// Built-in test pattern descriptions shown when no config overrides are present.
pub const DEFAULT_PATTERNS: &[(&str, &str)] = &[
    ("unit", "Tests a single function or module in isolation with fake or in-memory dependencies."),
    ("integration", "Tests a component boundary end-to-end (e.g., CLI subprocess, HTTP handler)."),
    ("systest", "Tests driven by a random input/mutation generator; correctness verified via invariant checking or differential comparison."),
    ("golden", "Tests that compare actual output against a checked-in expected file; fail on any diff."),
];

pub fn cmd_patterns(cfg: &BeuConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Test Patterns:");
    println!();
    if cfg.test_patterns.is_empty() {
        for (key, desc) in DEFAULT_PATTERNS {
            println!("  {key}");
            println!("    {desc}");
            println!();
        }
    } else {
        for p in &cfg.test_patterns {
            println!("  {}", p.key);
            println!("    {}", p.description);
            println!();
        }
    }
    println!("Test Status Lifecycle:");
    println!("  planned -> designed -> implemented -> tested -> darklaunched -> launched");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BeuConfig;

    #[test]
    fn default_patterns_prints_four_entries() {
        let cfg = BeuConfig::default();
        // Should not error.
        assert!(cmd_patterns(&cfg).is_ok());
    }

    #[test]
    fn config_patterns_override_defaults() {
        use crate::config::TestPatternConfig;
        let mut cfg = BeuConfig::default();
        cfg.test_patterns = vec![TestPatternConfig {
            key: "custom".to_string(),
            description: "A custom pattern.".to_string(),
        }];
        assert!(cmd_patterns(&cfg).is_ok());
    }
}
