use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use extism::{Manifest, Plugin, PluginBuilder};

use bea_sdk::{CommandInput, CommandOutput, PluginMetadata};

use crate::db::{EventLog, PluginDb, PluginLog};
use crate::host_functions::{self, HostContext};

/// A loaded Wasm plugin with its cached metadata.
struct LoadedPlugin {
    metadata: PluginMetadata,
    plugin: Plugin,
}

/// Registry of all discovered and loaded plugins.
pub struct PluginRegistry {
    plugins: HashMap<String, LoadedPlugin>,
    /// All plugin metadata (cached for skill export without needing mutable access).
    metadata_cache: Vec<PluginMetadata>,
    /// Event log for auditing all dispatched commands.
    event_log: Option<EventLog>,
}

impl PluginRegistry {
    /// Discover and load all `.wasm` files from `.bea/plugins/`.
    pub fn load(bea_dir: &Path, verbose: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let plugins_dir = bea_dir.join("plugins");
        if !plugins_dir.is_dir() {
            return Err(format!(
                "plugins directory not found: {}",
                plugins_dir.display()
            )
            .into());
        }

        let db = Arc::new(Mutex::new(PluginDb::new(bea_dir)));
        db.lock()
            .map_err(|e| format!("db lock: {e}"))?
            .ensure_dir()?;

        // Open plugin log store (shared across all plugins, non-fatal).
        let plugin_log = match PluginLog::open(bea_dir) {
            Ok(log) => Some(Arc::new(Mutex::new(log))),
            Err(e) => {
                if verbose {
                    eprintln!("verbose: failed to open plugin log: {e}");
                }
                None
            }
        };

        let project_root = bea_dir.parent().unwrap_or(bea_dir).to_path_buf();

        let mut plugins = HashMap::new();
        let mut metadata_cache = Vec::new();

        let wasm_files = discover_wasm_files(&plugins_dir)?;

        if verbose && wasm_files.is_empty() {
            eprintln!("verbose: no .wasm files found in {}", plugins_dir.display());
        }

        for wasm_path in wasm_files {
            if verbose {
                eprintln!("verbose: loading plugin {}", wasm_path.display());
            }
            match load_single_plugin(&wasm_path, bea_dir, &project_root, &db, &plugin_log) {
                Ok(loaded) => {
                    if verbose {
                        eprintln!(
                            "verbose: loaded {} v{} ({} commands)",
                            loaded.metadata.name,
                            loaded.metadata.version,
                            loaded.metadata.commands.len()
                        );
                    }
                    let name = loaded.metadata.name.clone();
                    metadata_cache.push(loaded.metadata.clone());
                    plugins.insert(name, loaded);
                }
                Err(e) => {
                    eprintln!(
                        "warning: failed to load plugin {}: {e}",
                        wasm_path.display()
                    );
                }
            }
        }

        // Open event log (non-fatal if it fails -- log a warning and continue).
        let event_log = match EventLog::open(bea_dir) {
            Ok(log) => Some(log),
            Err(e) => {
                if verbose {
                    eprintln!("verbose: failed to open event log: {e}");
                }
                None
            }
        };

        Ok(Self {
            plugins,
            metadata_cache,
            event_log,
        })
    }

    /// Get metadata for all loaded plugins.
    pub fn all_metadata(&self) -> &[PluginMetadata] {
        &self.metadata_cache
    }

    /// Access the event log (for the `bea events` command).
    pub fn event_log_mut(&mut self) -> Option<&mut EventLog> {
        self.event_log.as_mut()
    }

    /// Dispatch a command to the named plugin.
    pub fn dispatch(
        &mut self,
        plugin_name: &str,
        command: &str,
        args: &[String],
    ) -> Result<CommandOutput, Box<dyn std::error::Error>> {
        if !self.plugins.contains_key(plugin_name) {
            return if self.plugins.is_empty() {
                Err(format!(
                    "unknown plugin: '{plugin_name}' (no plugins loaded — place .wasm files in .bea/plugins/)"
                ).into())
            } else {
                let mut available: Vec<&str> =
                    self.plugins.keys().map(|s| s.as_str()).collect();
                available.sort();
                Err(format!(
                    "unknown plugin: '{plugin_name}' (available: {})",
                    available.join(", ")
                ).into())
            };
        }
        let loaded = self.plugins.get_mut(plugin_name)
            .ok_or_else(|| format!("plugin '{plugin_name}' not in registry"))?;

        let input = CommandInput {
            command: command.to_string(),
            args: args.to_vec(),
        };
        let input_json = serde_json::to_string(&input)?;

        let start = std::time::Instant::now();

        let result_json: String = loaded
            .plugin
            .call("run_command", &input_json)
            .map_err(|e| format!("plugin '{plugin_name}' run_command failed: {e}"))?;

        let output: CommandOutput = serde_json::from_str(&result_json)
            .map_err(|e| format!("plugin '{plugin_name}' returned invalid JSON: {e}"))?;

        let duration_ms = start.elapsed().as_millis() as i64;

        // Log the event (non-fatal on failure).
        let status_str = match output.status {
            bea_sdk::CommandStatus::Ok => "ok",
            bea_sdk::CommandStatus::Error => "error",
        };
        if let Some(ref mut log) = self.event_log {
            if let Err(e) = log.log(plugin_name, command, args, status_str, duration_ms) {
                eprintln!("warning: event log write failed: {e}");
            }
        }

        Ok(output)
    }
}

/// Find all `.wasm` files in the plugins directory.
fn discover_wasm_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "wasm") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Load a single Wasm plugin, call `metadata()`, and return the loaded plugin.
fn load_single_plugin(
    wasm_path: &Path,
    bea_dir: &Path,
    project_root: &Path,
    db: &Arc<Mutex<PluginDb>>,
    plugin_log: &Option<Arc<Mutex<PluginLog>>>,
) -> Result<LoadedPlugin, Box<dyn std::error::Error>> {
    let file_stem = wasm_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("invalid wasm filename")?
        .to_string();

    let context = HostContext {
        project_root: project_root.to_path_buf(),
        plugin_name: file_stem.clone(),
        db: Arc::clone(db),
        plugin_log: plugin_log.clone(),
    };

    let functions = host_functions::build_host_functions(context);
    let config = load_plugin_config(bea_dir, &file_stem);
    let manifest = Manifest::new([extism::Wasm::file(wasm_path)]).with_config(config.into_iter());

    let mut plugin = PluginBuilder::new(manifest)
        .with_wasi(false)
        .with_functions(functions)
        .build()?;

    // Call metadata() to get plugin self-description.
    let metadata_json: String = plugin.call("metadata", "")?;
    let metadata: PluginMetadata = serde_json::from_str(&metadata_json)
        .map_err(|e| format!("plugin '{}' metadata() returned invalid JSON: {e}", file_stem))?;

    // Sanity: plugin name must match filename.
    if metadata.name != file_stem {
        return Err(format!(
            "plugin name mismatch: file is '{}' but metadata says '{}'",
            file_stem, metadata.name
        )
        .into());
    }

    Ok(LoadedPlugin { metadata, plugin })
}

/// Load per-plugin config from `.bea/config/<plugin_name>.toml`.
/// Returns an empty map if no config file exists.
/// Flattens nested TOML tables into dot-separated keys (e.g. `section.key = "value"`).
fn load_plugin_config(bea_dir: &Path, plugin_name: &str) -> BTreeMap<String, String> {
    let config_path = bea_dir.join(format!("config/{plugin_name}.toml"));
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return BTreeMap::new(),
    };

    let table: toml::Table = match content.parse() {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "warning: failed to parse {}: {e}",
                config_path.display()
            );
            return BTreeMap::new();
        }
    };

    let mut result = BTreeMap::new();
    flatten_toml("", &toml::Value::Table(table), &mut result);
    result
}

/// Recursively flatten a TOML value into dot-separated string key-value pairs.
fn flatten_toml(prefix: &str, value: &toml::Value, out: &mut BTreeMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_toml(&key, v, out);
            }
        }
        toml::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}
