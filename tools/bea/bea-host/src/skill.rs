use std::path::Path;

use bea_sdk::PluginMetadata;

use crate::plugin_manager::PluginRegistry;

/// Generate `.bea/skill.md` from all loaded plugin metadata.
pub fn export(
    bea_dir: &Path,
    registry: &PluginRegistry,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_name = detect_project_name(bea_dir);
    let content = render_skill_md(&project_name, registry.all_metadata());

    let skill_path = bea_dir.join("skill.md");
    std::fs::write(&skill_path, &content)?;
    if !quiet {
        println!("Wrote {}", skill_path.display());
    }
    Ok(())
}

/// Print all available commands as JSON to stdout.
pub fn info(registry: &PluginRegistry) -> Result<(), Box<dyn std::error::Error>> {
    let output = serde_json::json!({
        "system_commands": [
            {
                "name": "bea skill export",
                "description": "Regenerate the skill.md manifest from loaded plugins."
            },
            {
                "name": "bea skill info",
                "description": "Print this JSON representation of all commands."
            },
            {
                "name": "bea init",
                "description": "Initialize a new .bea project directory."
            },
            {
                "name": "bea install <path.wasm>",
                "description": "Install a plugin from a local .wasm file."
            },
            {
                "name": "bea list",
                "description": "List installed plugins and their commands."
            },
            {
                "name": "bea uninstall <name>",
                "description": "Remove a plugin (--purge to also delete its database)."
            },
            {
                "name": "bea status",
                "description": "Show project overview (plugins, data, events)."
            },
            {
                "name": "bea events",
                "description": "Show recent event log entries for debugging."
            },
            {
                "name": "bea logs",
                "description": "Show recent plugin log entries (from host_log calls)."
            },
            {
                "name": "bea run <script>",
                "description": "Run a batch script of plugin commands."
            },
            {
                "name": "bea export <plugin>",
                "description": "Export plugin data as JSON."
            },
            {
                "name": "bea import <plugin> <file.json>",
                "description": "Import data from JSON into plugin database."
            },
            {
                "name": "bea reset <plugin> --force",
                "description": "Drop all tables in a plugin's database."
            },
            {
                "name": "bea config <plugin> [key] [value]",
                "description": "View or set plugin configuration."
            },
            {
                "name": "bea version",
                "description": "Show version and build information."
            },
            {
                "name": "bea completions <shell>",
                "description": "Generate shell completions (bash, zsh, fish)."
            }
        ],
        "plugins": registry.all_metadata(),
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn render_skill_md(project_name: &str, plugins: &[PluginMetadata]) -> String {
    let mut md = String::new();

    md.push_str(&format!("# Agent Skill Manifest: {project_name}\n\n"));

    md.push_str("## System Commands (Built-in)\n\n");
    md.push_str("- `bea skill export`: Regenerate this document from loaded plugins.\n");
    md.push_str("- `bea skill info`: JSON representation of all available commands.\n");
    md.push_str("- `bea init`: Initialize a new .bea project directory.\n");
    md.push_str("- `bea install <path.wasm>`: Install a plugin from a local .wasm file.\n");
    md.push_str("- `bea list`: List installed plugins and their commands.\n");
    md.push_str("- `bea uninstall <name> [--purge]`: Remove a plugin (--purge deletes its database).\n");
    md.push_str("- `bea status`: Show project overview.\n");
    md.push_str("- `bea events [-n <limit>] [--plugin <name>]`: Show recent event log entries.\n");
    md.push_str("- `bea logs [-n <limit>] [--plugin <name>] [--level <level>]`: Show recent plugin log entries.\n");
    md.push_str("- `bea run <script> [--fail-fast]`: Run a batch script of plugin commands.\n");
    md.push_str("- `bea export <plugin>`: Export plugin data as JSON (or `--all` for all plugins).\n");
    md.push_str("- `bea import <plugin> <file.json>`: Import data from JSON into plugin database.\n");
    md.push_str("- `bea reset <plugin> --force`: Drop all tables in a plugin's database.\n");
    md.push_str("- `bea config <plugin> [key] [value] [--delete]`: View or set plugin configuration.\n");
    md.push_str("- `bea version`: Show version and build information.\n");
    md.push_str("- `bea completions <shell>`: Generate shell completions (bash, zsh, fish).\n");
    md.push('\n');

    for plugin in plugins {
        md.push_str(&format!(
            "## {} (v{})\n\n",
            titlecase(&plugin.name),
            plugin.version
        ));
        md.push_str(&format!("{}\n\n", plugin.description));

        for cmd in &plugin.commands {
            let args_str = cmd
                .args
                .iter()
                .map(|a| {
                    if a.required {
                        format!("<{}>", a.name)
                    } else {
                        format!("[{}]", a.name)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            if args_str.is_empty() {
                md.push_str(&format!(
                    "- `bea {} {}`: {}\n",
                    plugin.name, cmd.name, cmd.description
                ));
            } else {
                md.push_str(&format!(
                    "- `bea {} {} {}`: {}\n",
                    plugin.name, cmd.name, args_str, cmd.description
                ));
            }
        }
        md.push('\n');
    }

    md
}

fn titlecase(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn detect_project_name(bea_dir: &Path) -> String {
    bea_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bea_sdk::{ArgDef, CommandDef};

    #[test]
    fn render_empty_plugins() {
        let md = render_skill_md("test-project", &[]);
        assert!(md.contains("# Agent Skill Manifest: test-project"));
        assert!(md.contains("bea skill export"));
    }

    #[test]
    fn titlecase_basic() {
        assert_eq!(titlecase("journal"), "Journal");
        assert_eq!(titlecase("agile"), "Agile");
    }

    #[test]
    fn titlecase_empty_and_single() {
        assert_eq!(titlecase(""), "");
        assert_eq!(titlecase("a"), "A");
    }

    #[test]
    fn detect_project_name_from_parent() {
        let dir = std::path::Path::new("/tmp/my-project/.bea");
        let name = detect_project_name(dir);
        assert_eq!(name, "my-project");
    }

    #[test]
    fn render_with_plugin() {
        let plugins = vec![PluginMetadata {
            name: "journal".into(),
            version: "0.1.0".into(),
            description: "Agent interaction ledger.".into(),
            commands: vec![
                CommandDef {
                    name: "open".into(),
                    description: "Start a new session.".into(),
                    args: vec![],
                },
                CommandDef {
                    name: "log".into(),
                    description: "Record a message.".into(),
                    args: vec![ArgDef {
                        name: "message".into(),
                        description: "The message to record.".into(),
                        required: true,
                    }],
                },
            ],
        }];
        let md = render_skill_md("my-project", &plugins);
        assert!(md.contains("## Journal (v0.1.0)"));
        assert!(md.contains("`bea journal open`"));
        assert!(md.contains("`bea journal log <message>`"));
    }
}
