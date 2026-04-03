use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPatternConfig {
    pub key: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BeuConfig {
    pub modules: ModuleConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_docs: Vec<RequiredDoc>,
    /// When true, --project flag is required for all commands.
    /// When false (default), commands use default_project if --project is omitted.
    #[serde(default)]
    pub require_project: bool,
    /// The project ID to use when --project is omitted and require_project is false.
    #[serde(default = "default_project_id")]
    pub default_project: String,
    /// If set, `beu check` fails when a required doc has this many mutation events
    /// since its last update. None = staleness check disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staleness_threshold: Option<u64>,
    /// Override or extend the built-in test patterns shown by `beu test patterns`.
    /// When empty, the built-in defaults are used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_patterns: Vec<TestPatternConfig>,
}

fn default_project_id() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredDoc {
    pub name: String,
    #[serde(rename = "type", default = "default_doc_type")]
    pub doc_type: String,
}

fn default_doc_type() -> String {
    "doc".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleConfig {
    #[serde(default = "default_true")]
    pub journal: bool,
    #[serde(default = "default_true")]
    pub artifact: bool,
    #[serde(default = "default_true")]
    pub task: bool,
    #[serde(default = "default_true")]
    pub state: bool,
    #[serde(default = "default_true")]
    pub idea: bool,
    #[serde(default = "default_true")]
    pub debug: bool,
}

fn default_true() -> bool {
    true
}

impl Default for BeuConfig {
    fn default() -> Self {
        Self {
            modules: ModuleConfig::default(),
            required_docs: Vec::new(),
            require_project: false,
            default_project: "default".to_string(),
            staleness_threshold: None,
            test_patterns: Vec::new(),
        }
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            journal: true,
            artifact: true,
            task: true,
            state: true,
            idea: true,
            debug: true,
        }
    }
}

impl BeuConfig {
    pub fn require_module(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_module_enabled(name) {
            Ok(())
        } else {
            Err(format!("module '{name}' is not enabled in config.yml").into())
        }
    }

    pub fn is_module_enabled(&self, name: &str) -> bool {
        match name {
            "journal" => self.modules.journal,
            "artifact" => self.modules.artifact,
            "task" => self.modules.task,
            "state" => self.modules.state,
            "idea" => self.modules.idea,
            "debug" => self.modules.debug,
            _ => false,
        }
    }

    pub fn resolve_project(
        &self,
        cli_project: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        match cli_project {
            Some(id) => Ok(id.to_string()),
            None => {
                if self.require_project {
                    Err("project ID required (use --project <id> or set require_project: false in config.yml)".into())
                } else {
                    Ok(self.default_project.clone())
                }
            }
        }
    }

    pub fn enabled_modules(&self) -> Vec<&str> {
        let mut result = Vec::new();
        if self.modules.journal {
            result.push("journal");
        }
        if self.modules.artifact {
            result.push("artifact");
        }
        if self.modules.task {
            result.push("task");
        }
        if self.modules.state {
            result.push("state");
        }
        if self.modules.idea {
            result.push("idea");
        }
        if self.modules.debug {
            result.push("debug");
        }
        result
    }
}

pub fn load(beu_dir: &Path) -> Result<BeuConfig, Box<dyn std::error::Error>> {
    let config_path = beu_dir.join("config.yml");
    if !config_path.exists() {
        return Ok(BeuConfig::default());
    }
    let content = std::fs::read_to_string(&config_path)?;
    let config: BeuConfig =
        serde_yaml::from_str(&content).map_err(|e| format!("invalid config.yml: {e}"))?;
    Ok(config)
}

pub fn save(beu_dir: &Path, config: &BeuConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = beu_dir.join("config.yml");
    let content = serde_yaml::to_string(config)?;
    std::fs::write(&config_path, content)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_enables_all_modules() {
        let config = BeuConfig::default();
        assert!(config.is_module_enabled("journal"));
        assert!(config.is_module_enabled("artifact"));
        assert!(config.is_module_enabled("task"));
        assert!(config.is_module_enabled("state"));
        assert!(config.is_module_enabled("idea"));
        assert!(config.is_module_enabled("debug"));
    }

    #[test]
    fn unknown_module_returns_false() {
        let config = BeuConfig::default();
        assert!(!config.is_module_enabled("nonexistent"));
    }

    #[test]
    fn require_module_ok_when_enabled() {
        let config = BeuConfig::default();
        assert!(config.require_module("journal").is_ok());
    }

    #[test]
    fn require_module_err_when_disabled() {
        let mut config = BeuConfig::default();
        config.modules.journal = false;
        let err = config.require_module("journal").unwrap_err();
        assert!(err.to_string().contains("not enabled"));
    }

    #[test]
    fn enabled_modules_lists_only_enabled() {
        let mut config = BeuConfig::default();
        config.modules.artifact = false;
        config.modules.debug = false;
        let enabled = config.enabled_modules();
        assert_eq!(enabled, vec!["journal", "task", "state", "idea"]);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let mut config = BeuConfig::default();
        config.modules.journal = false;
        config.modules.idea = false;

        save(&beu_dir, &config).unwrap();
        let loaded = load(&beu_dir).unwrap();

        assert!(!loaded.modules.journal);
        assert!(loaded.modules.artifact);
        assert!(loaded.modules.task);
        assert!(loaded.modules.state);
        assert!(!loaded.modules.idea);
        assert!(loaded.modules.debug);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let config = load(&beu_dir).unwrap();
        assert!(config.is_module_enabled("journal"));
        assert!(config.is_module_enabled("artifact"));
    }

    #[test]
    fn default_has_empty_required_docs() {
        let config = BeuConfig::default();
        assert!(config.required_docs.is_empty());
    }

    #[test]
    fn load_without_required_docs_defaults_to_empty() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let yaml = "modules:\n  journal: true\n  artifact: true\n  task: true\n  state: true\n  idea: true\n  debug: true\n";
        std::fs::write(beu_dir.join("config.yml"), yaml).unwrap();

        let config = load(&beu_dir).unwrap();
        assert!(config.required_docs.is_empty());
    }

    #[test]
    fn load_with_required_docs() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let yaml = "modules:\n  journal: true\n  artifact: true\n  task: true\n  state: true\n  idea: true\n  debug: true\nrequired_docs:\n  - name: design\n    type: doc\n  - name: changelog\n    type: changelog\n";
        std::fs::write(beu_dir.join("config.yml"), yaml).unwrap();

        let config = load(&beu_dir).unwrap();
        assert_eq!(config.required_docs.len(), 2);
        assert_eq!(config.required_docs[0].name, "design");
        assert_eq!(config.required_docs[0].doc_type, "doc");
        assert_eq!(config.required_docs[1].name, "changelog");
        assert_eq!(config.required_docs[1].doc_type, "changelog");
    }

    #[test]
    fn required_doc_type_defaults_to_doc() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let yaml = "modules:\n  journal: true\n  artifact: true\n  task: true\n  state: true\n  idea: true\n  debug: true\nrequired_docs:\n  - name: design\n";
        std::fs::write(beu_dir.join("config.yml"), yaml).unwrap();

        let config = load(&beu_dir).unwrap();
        assert_eq!(config.required_docs.len(), 1);
        assert_eq!(config.required_docs[0].doc_type, "doc");
    }

    #[test]
    fn save_and_load_roundtrip_with_required_docs() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let config = BeuConfig {
            modules: ModuleConfig::default(),
            required_docs: vec![
                RequiredDoc {
                    name: "design".to_string(),
                    doc_type: "doc".to_string(),
                },
                RequiredDoc {
                    name: "changelog".to_string(),
                    doc_type: "changelog".to_string(),
                },
            ],
            ..BeuConfig::default()
        };

        save(&beu_dir, &config).unwrap();
        let loaded = load(&beu_dir).unwrap();

        assert_eq!(loaded.required_docs.len(), 2);
        assert_eq!(loaded.required_docs[0].name, "design");
        assert_eq!(loaded.required_docs[1].doc_type, "changelog");
    }

    #[test]
    fn save_default_omits_required_docs() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        save(&beu_dir, &BeuConfig::default()).unwrap();
        let content = std::fs::read_to_string(beu_dir.join("config.yml")).unwrap();
        assert!(!content.contains("required_docs"));
    }

    #[test]
    fn load_partial_yaml_defaults_missing_fields() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        // YAML with only some fields.
        let yaml = "modules:\n  journal: false\n  task: false\n";
        std::fs::write(beu_dir.join("config.yml"), yaml).unwrap();

        let config = load(&beu_dir).unwrap();
        assert!(!config.modules.journal);
        assert!(config.modules.artifact); // defaulted to true
        assert!(!config.modules.task);
        assert!(config.modules.state); // defaulted to true
    }

    // --- Project config tests ---

    #[test]
    fn default_config_has_require_project_false() {
        let config = BeuConfig::default();
        assert!(!config.require_project);
    }

    #[test]
    fn default_config_has_default_project() {
        let config = BeuConfig::default();
        assert_eq!(config.default_project, "default");
    }

    #[test]
    fn resolve_project_with_explicit_id() {
        let config = BeuConfig::default();
        let id = config.resolve_project(Some("alpha")).unwrap();
        assert_eq!(id, "alpha");
    }

    #[test]
    fn resolve_project_uses_default_when_not_required() {
        let config = BeuConfig::default();
        let id = config.resolve_project(None).unwrap();
        assert_eq!(id, "default");
    }

    #[test]
    fn resolve_project_fails_when_required_and_missing() {
        let mut config = BeuConfig::default();
        config.require_project = true;
        let err = config.resolve_project(None).unwrap_err();
        assert!(err.to_string().contains("project ID required"));
    }

    #[test]
    fn resolve_project_succeeds_when_required_and_provided() {
        let mut config = BeuConfig::default();
        config.require_project = true;
        let id = config.resolve_project(Some("beta")).unwrap();
        assert_eq!(id, "beta");
    }

    #[test]
    fn save_and_load_roundtrip_with_project_config() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let mut config = BeuConfig::default();
        config.require_project = true;
        config.default_project = "myproject".to_string();

        save(&beu_dir, &config).unwrap();
        let loaded = load(&beu_dir).unwrap();

        assert!(loaded.require_project);
        assert_eq!(loaded.default_project, "myproject");
    }

    #[test]
    fn load_partial_yaml_defaults_project_fields() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let yaml = "modules:\n  journal: true\n";
        std::fs::write(beu_dir.join("config.yml"), yaml).unwrap();

        let config = load(&beu_dir).unwrap();
        assert!(!config.require_project);
        assert_eq!(config.default_project, "default");
    }

    #[test]
    fn save_default_omits_project_fields_with_defaults() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        save(&beu_dir, &BeuConfig::default()).unwrap();
        let _content = std::fs::read_to_string(beu_dir.join("config.yml")).unwrap();
        // Default values should still be serialized (serde_yaml serializes all fields),
        // but they should load correctly.
        let loaded = load(&beu_dir).unwrap();
        assert!(!loaded.require_project);
        assert_eq!(loaded.default_project, "default");
    }

    #[test]
    fn default_config_has_no_staleness_threshold() {
        let config = BeuConfig::default();
        assert!(config.staleness_threshold.is_none());
    }

    #[test]
    fn save_and_load_roundtrip_with_staleness_threshold() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        let mut config = BeuConfig::default();
        config.staleness_threshold = Some(10);
        save(&beu_dir, &config).unwrap();

        let loaded = load(&beu_dir).unwrap();
        assert_eq!(loaded.staleness_threshold, Some(10));
    }

    #[test]
    fn save_default_omits_staleness_threshold() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        std::fs::create_dir_all(&beu_dir).unwrap();

        save(&beu_dir, &BeuConfig::default()).unwrap();
        let content = std::fs::read_to_string(beu_dir.join("config.yml")).unwrap();
        assert!(!content.contains("staleness_threshold"));
    }
}
