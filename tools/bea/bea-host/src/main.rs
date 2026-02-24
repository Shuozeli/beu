use bea_host::plugin_manager;
use bea_host::skill;

use std::path::PathBuf;
use std::process;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use plugin_manager::PluginRegistry;

/// bea - A universal sandboxed Wasm plugin host for AI agents.
#[derive(Parser)]
#[command(name = "bea", version, about)]
struct Cli {
    /// Path to the .bea directory (default: .bea in current or ancestor directory).
    #[arg(long, global = true)]
    bea_dir: Option<PathBuf>,

    /// Show detailed output (plugin loading, host function calls).
    #[arg(long, short, global = true)]
    verbose: bool,

    /// Suppress all non-essential output.
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage the agent skill manifest.
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Initialize a new .bea project directory.
    Init,
    /// Install a plugin from a local .wasm file.
    Install {
        /// Path to the .wasm file to install.
        path: PathBuf,
    },
    /// List installed plugins and their commands.
    List,
    /// Uninstall a plugin by name.
    Uninstall {
        /// Name of the plugin to remove (e.g., "journal").
        name: String,
        /// Also delete the plugin's database file.
        #[arg(long)]
        purge: bool,
    },
    /// Show project status overview.
    Status,
    /// Generate shell completions.
    Completions {
        /// Shell to generate completions for.
        shell: Shell,
    },
    /// Show recent event log entries for debugging.
    Events {
        /// Maximum number of events to show (default: 20).
        #[arg(long, short = 'n', default_value = "20")]
        limit: usize,
        /// Filter events by plugin name.
        #[arg(long, short)]
        plugin: Option<String>,
    },
    /// Show recent plugin log entries (from host_log calls).
    Logs {
        /// Maximum number of log entries to show (default: 50).
        #[arg(long, short = 'n', default_value = "50")]
        limit: usize,
        /// Filter by plugin name.
        #[arg(long, short)]
        plugin: Option<String>,
        /// Filter by log level (info, warn, error, debug).
        #[arg(long, short)]
        level: Option<String>,
    },
    /// Run a batch script of plugin commands.
    Run {
        /// Path to the script file (.bea or plain text).
        script: PathBuf,
        /// Stop on first error (default: continue).
        #[arg(long)]
        fail_fast: bool,
    },
    /// Export plugin data as JSON.
    Export {
        /// Plugin name (or --all for all plugins).
        plugin: Option<String>,
        /// Export all plugin databases.
        #[arg(long)]
        all: bool,
    },
    /// Import data from a JSON file into a plugin's database.
    Import {
        /// Plugin name.
        plugin: String,
        /// Path to JSON file (format: same as `bea export` output).
        file: PathBuf,
    },
    /// Reset a plugin's database (drop all tables).
    Reset {
        /// Plugin name.
        plugin: String,
        /// Skip confirmation prompt.
        #[arg(long)]
        force: bool,
    },
    /// View or set plugin configuration.
    Config {
        /// Plugin name.
        plugin: String,
        /// Key to get or set. If omitted, shows all config.
        key: Option<String>,
        /// Value to set. Requires a key.
        value: Option<String>,
        /// Delete the key instead of setting it.
        #[arg(long)]
        delete: bool,
    },
    /// Show version and build information.
    Version,
    /// Run a plugin command: bea <plugin> <command> [args...]
    #[command(external_subcommand)]
    Plugin(Vec<String>),
}

#[derive(Subcommand)]
enum SkillAction {
    /// Generate .bea/skill.md from loaded plugins.
    Export,
    /// Print all available commands as JSON.
    Info,
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose && cli.quiet {
        eprintln!("error: --verbose and --quiet are mutually exclusive");
        process::exit(1);
    }

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let verbose = cli.verbose;
    let quiet = cli.quiet;

    match cli.command {
        Commands::Init => {
            let root = std::env::current_dir()?;
            cmd_init(&root, quiet)?;
            Ok(())
        }
        Commands::Skill { action } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            let registry = PluginRegistry::load(&bea_dir, verbose)?;
            match action {
                SkillAction::Export => skill::export(&bea_dir, &registry, quiet),
                SkillAction::Info => skill::info(&registry),
            }
        }
        Commands::Install { path } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_install(&bea_dir, &path, verbose, quiet)
        }
        Commands::List => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_list(&bea_dir, verbose)
        }
        Commands::Uninstall { name, purge } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_uninstall(&bea_dir, &name, purge, verbose, quiet)
        }
        Commands::Status => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_status(&bea_dir, verbose)
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "bea", &mut std::io::stdout());
            Ok(())
        }
        Commands::Events { limit, plugin } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_events(&bea_dir, limit, plugin.as_deref())
        }
        Commands::Logs {
            limit,
            plugin,
            level,
        } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_logs(&bea_dir, limit, plugin.as_deref(), level.as_deref())
        }
        Commands::Run { script, fail_fast } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_run(&bea_dir, &script, fail_fast, verbose, quiet)
        }
        Commands::Export { plugin, all } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_export(&bea_dir, plugin.as_deref(), all)
        }
        Commands::Import { plugin, file } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_import(&bea_dir, &plugin, &file, quiet)
        }
        Commands::Reset { plugin, force } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_reset(&bea_dir, &plugin, force, quiet)
        }
        Commands::Config {
            plugin,
            key,
            value,
            delete,
        } => {
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;
            cmd_config(&bea_dir, &plugin, key.as_deref(), value.as_deref(), delete)
        }
        Commands::Version => {
            cmd_version(cli.bea_dir);
            Ok(())
        }
        Commands::Plugin(args) => {
            if args.is_empty() {
                return Err("no plugin command specified".into());
            }
            let plugin_name = &args[0];
            let bea_dir = resolve_bea_dir(cli.bea_dir)?;

            let (command, cmd_args) = if args.len() > 1 {
                (args[1].as_str(), &args[2..])
            } else {
                // No command given -- show plugin help.
                let registry = PluginRegistry::load(&bea_dir, verbose)?;
                return cmd_plugin_help(&registry, plugin_name);
            };

            let mut registry = PluginRegistry::load(&bea_dir, verbose)?;
            let output = registry.dispatch(plugin_name, command, cmd_args)?;

            match output.status {
                bea_sdk::CommandStatus::Ok => {
                    if !output.message.is_empty() {
                        println!("{}", output.message);
                    }
                    if let Some(data) = &output.data {
                        println!("{}", serde_json::to_string_pretty(data)?);
                    }
                }
                bea_sdk::CommandStatus::Error => {
                    eprintln!("plugin error: {}", output.message);
                    process::exit(1);
                }
            }
            Ok(())
        }
    }
}

fn cmd_init(root: &std::path::Path, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    let bea_dir = root.join(".bea");
    if bea_dir.exists() {
        return Err(format!(".bea directory already exists at {}", bea_dir.display()).into());
    }
    std::fs::create_dir_all(bea_dir.join("plugins"))?;
    std::fs::create_dir_all(bea_dir.join("data"))?;
    std::fs::create_dir_all(bea_dir.join("config"))?;

    // Generate initial skill.md with system commands only.
    let registry = PluginRegistry::load(&bea_dir, false)?;
    skill::export(&bea_dir, &registry, true)?;

    if !quiet {
        println!("Initialized .bea at {}", bea_dir.display());
        println!("  plugins/  - place .wasm plugin files here");
        println!("  data/     - per-plugin SQLite databases");
        println!("  config/   - per-plugin TOML configuration");
        println!("  skill.md  - agent skill manifest");
    }
    Ok(())
}

/// Walk up from CWD to find .bea/, or use the explicit --bea-dir flag.
fn resolve_bea_dir(explicit: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(dir) = explicit {
        if !dir.exists() {
            return Err(format!(".bea directory not found at {}", dir.display()).into());
        }
        return Ok(dir);
    }

    let mut current = std::env::current_dir()?;
    loop {
        let candidate = current.join(".bea");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !current.pop() {
            break;
        }
    }

    Err("no .bea directory found (run `bea init` first)".into())
}

fn cmd_install(
    bea_dir: &std::path::Path,
    wasm_path: &std::path::Path,
    verbose: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !wasm_path.exists() {
        return Err(format!("file not found: {}", wasm_path.display()).into());
    }
    if wasm_path.extension().and_then(|e| e.to_str()) != Some("wasm") {
        return Err("file must have .wasm extension".into());
    }

    let file_name = wasm_path
        .file_name()
        .ok_or("invalid file path")?
        .to_string_lossy()
        .to_string();
    let plugin_name = wasm_path
        .file_stem()
        .ok_or("invalid file name")?
        .to_string_lossy()
        .to_string();

    let dest = bea_dir.join("plugins").join(&file_name);

    if verbose {
        eprintln!("verbose: validating plugin {}", wasm_path.display());
    }

    if dest.exists() && verbose {
        eprintln!("verbose: overwriting existing {}", dest.display());
    }

    std::fs::copy(wasm_path, &dest)?;

    // Validate by loading the registry with the new plugin.
    let registry = PluginRegistry::load(bea_dir, false)?;
    let meta = registry
        .all_metadata()
        .iter()
        .find(|m| m.name == plugin_name);

    match meta {
        Some(m) => {
            if !quiet {
                println!(
                    "Installed {} v{} ({} commands)",
                    m.name,
                    m.version,
                    m.commands.len()
                );
            }

            // Auto-regenerate skill.md.
            skill::export(bea_dir, &registry, true)?;
            if !quiet {
                println!("Updated skill.md");
            }
        }
        None => {
            // Plugin loaded but name mismatch -- remove and error.
            let _ = std::fs::remove_file(&dest);
            return Err(format!(
                "plugin file name '{}' does not match any loaded plugin metadata name",
                plugin_name
            )
            .into());
        }
    }

    Ok(())
}

fn cmd_status(
    bea_dir: &std::path::Path,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = PluginRegistry::load(bea_dir, verbose)?;
    let plugins = registry.all_metadata();
    let db = bea_host::db::PluginDb::new(bea_dir);

    let total_commands: usize = plugins.iter().map(|p| p.commands.len()).sum();
    let total_db_size: u64 = plugins
        .iter()
        .filter_map(|p| db.db_size(&p.name))
        .sum();

    println!("bea project: {}", bea_dir.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("?"));
    println!("  plugins:  {} ({} commands)", plugins.len(), total_commands);

    if total_db_size > 0 {
        let size_str = if total_db_size >= 1024 * 1024 {
            format!("{:.1}MB", total_db_size as f64 / (1024.0 * 1024.0))
        } else if total_db_size >= 1024 {
            format!("{:.1}KB", total_db_size as f64 / 1024.0)
        } else {
            format!("{total_db_size}B")
        };
        println!("  data:     {size_str}");
    } else {
        println!("  data:     (empty)");
    }

    // Recent event activity.
    match bea_host::db::EventLog::open(bea_dir) {
        Ok(mut log) => {
            let recent = log.recent(1, None)?;
            if let Some(last) = recent.first() {
                println!("  last activity: {} {} {} ({})", last.timestamp, last.plugin, last.command, last.status);
            } else {
                println!("  last activity: (none)");
            }
            let count = log.count()?;
            println!("  total events: {count}");
        }
        Err(_) => {
            println!("  events: (no log)");
        }
    }

    if verbose {
        println!();
        for plugin in plugins {
            let db_size = db.db_size(&plugin.name);
            let size_str = match db_size {
                Some(s) if s > 0 => format!(" ({}B)", s),
                _ => String::new(),
            };
            println!("  {} v{} - {} commands{}", plugin.name, plugin.version, plugin.commands.len(), size_str);
        }
    }

    Ok(())
}

fn cmd_plugin_help(
    registry: &PluginRegistry,
    plugin_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let meta = registry
        .all_metadata()
        .iter()
        .find(|m| m.name == plugin_name);

    match meta {
        Some(m) => {
            println!("{} v{}", m.name, m.version);
            println!("{}", m.description);
            println!();
            println!("Commands:");
            for cmd in &m.commands {
                let args_str: Vec<String> = cmd
                    .args
                    .iter()
                    .map(|a| {
                        if a.required {
                            format!("<{}>", a.name)
                        } else {
                            format!("[{}]", a.name)
                        }
                    })
                    .collect();
                if args_str.is_empty() {
                    println!("  bea {} {:<16} {}", m.name, cmd.name, cmd.description);
                } else {
                    println!(
                        "  bea {} {} {:<10} {}",
                        m.name,
                        cmd.name,
                        args_str.join(" "),
                        cmd.description
                    );
                }
            }
            Ok(())
        }
        None => {
            if registry.all_metadata().is_empty() {
                Err(format!(
                    "unknown plugin: '{plugin_name}' (no plugins loaded — place .wasm files in .bea/plugins/)"
                ).into())
            } else {
                let mut available: Vec<&str> = registry
                    .all_metadata()
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect();
                available.sort();
                Err(format!(
                    "unknown plugin: '{plugin_name}' (available: {})",
                    available.join(", ")
                ).into())
            }
        }
    }
}

fn cmd_uninstall(
    bea_dir: &std::path::Path,
    name: &str,
    purge: bool,
    verbose: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let wasm_path = bea_dir.join(format!("plugins/{name}.wasm"));
    if !wasm_path.exists() {
        return Err(format!("plugin '{name}' not found (no {name}.wasm in plugins/)").into());
    }

    std::fs::remove_file(&wasm_path)?;
    if !quiet {
        println!("Removed {name}.wasm");
    }

    if purge {
        let db_path = bea_dir.join(format!("data/{name}.db"));
        if db_path.exists() {
            std::fs::remove_file(&db_path)?;
            if !quiet {
                println!("Purged {name}.db");
            }
        }
        // Also remove WAL/SHM files if present.
        for suffix in &["-wal", "-shm"] {
            let wal_path = bea_dir.join(format!("data/{name}.db{suffix}"));
            if wal_path.exists() {
                let _ = std::fs::remove_file(&wal_path);
            }
        }
    }

    // Regenerate skill.md with the remaining plugins.
    let registry = PluginRegistry::load(bea_dir, verbose)?;
    skill::export(bea_dir, &registry, true)?;
    if !quiet {
        println!("Updated skill.md");
    }

    Ok(())
}

fn cmd_events(
    bea_dir: &std::path::Path,
    limit: usize,
    plugin_filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut log = bea_host::db::EventLog::open(bea_dir)?;
    let events = log.recent(limit, plugin_filter)?;

    if events.is_empty() {
        println!("No events recorded yet.");
        return Ok(());
    }

    println!(
        "{:<5} {:<22} {:<12} {:<12} {:<20} {:<8}",
        "ID", "TIMESTAMP", "PLUGIN", "COMMAND", "ARGS", "STATUS"
    );
    println!("{}", "-".repeat(80));

    for event in &events {
        let args_display = if event.args.len() > 18 {
            format!("{}...", &event.args[..18])
        } else {
            event.args.clone()
        };
        println!(
            "{:<5} {:<22} {:<12} {:<12} {:<20} {:<8}",
            event.id, event.timestamp, event.plugin, event.command, args_display, event.status
        );
    }

    println!("\n{} event(s) shown", events.len());
    Ok(())
}

fn cmd_run(
    bea_dir: &std::path::Path,
    script: &std::path::Path,
    fail_fast: bool,
    verbose: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !script.exists() {
        return Err(format!("script not found: {}", script.display()).into());
    }

    let content = std::fs::read_to_string(script)?;
    let lines: Vec<&str> = content.lines().collect();

    let mut registry = PluginRegistry::load(bea_dir, verbose)?;
    let mut errors = 0u32;
    let mut executed = 0u32;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse: <plugin> <command> [args...]
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 2 {
            let msg = format!("line {}: invalid command (expected: <plugin> <command> [args...]): {trimmed}", i + 1);
            if fail_fast {
                return Err(msg.into());
            }
            eprintln!("error: {msg}");
            errors += 1;
            continue;
        }

        let plugin_name = parts[0];
        let command = parts[1];
        let args: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();

        if !quiet {
            println!(">> {} {} {}", plugin_name, command, args.join(" "));
        }

        match registry.dispatch(plugin_name, command, &args) {
            Ok(output) => {
                executed += 1;
                match output.status {
                    bea_sdk::CommandStatus::Ok => {
                        if !quiet && !output.message.is_empty() {
                            println!("{}", output.message);
                        }
                    }
                    bea_sdk::CommandStatus::Error => {
                        errors += 1;
                        eprintln!("plugin error: {}", output.message);
                        if fail_fast {
                            return Err(format!(
                                "script failed at line {} (executed {executed}, {errors} error(s))",
                                i + 1
                            )
                            .into());
                        }
                    }
                }
            }
            Err(e) => {
                errors += 1;
                eprintln!("error at line {}: {e}", i + 1);
                if fail_fast {
                    return Err(format!(
                        "script failed at line {} (executed {executed}, {errors} error(s))",
                        i + 1
                    )
                    .into());
                }
            }
        }
    }

    if !quiet {
        println!("\nScript complete: {executed} executed, {errors} error(s)");
    }

    if errors > 0 && fail_fast {
        Err(format!("{errors} error(s) in script").into())
    } else {
        Ok(())
    }
}

fn cmd_export(
    bea_dir: &std::path::Path,
    plugin: Option<&str>,
    all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut db = bea_host::db::PluginDb::new(bea_dir);
    db.ensure_dir()?;

    match (plugin, all) {
        (Some(name), false) => {
            let data = db.export_plugin(name)?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(())
        }
        (None, true) => {
            let names = db.list_plugin_dbs()?;
            if names.is_empty() {
                println!("{{}}");
                return Ok(());
            }
            let mut result = serde_json::Map::new();
            for name in &names {
                match db.export_plugin(name) {
                    Ok(data) => {
                        result.insert(name.clone(), data);
                    }
                    Err(e) => {
                        eprintln!("warning: failed to export {name}: {e}");
                    }
                }
            }
            println!("{}", serde_json::to_string_pretty(&serde_json::Value::Object(result))?);
            Ok(())
        }
        (None, false) => Err("specify a plugin name or use --all".into()),
        (Some(_), true) => Err("cannot use --all with a specific plugin name".into()),
    }
}

fn cmd_logs(
    bea_dir: &std::path::Path,
    limit: usize,
    plugin_filter: Option<&str>,
    level_filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut log = bea_host::db::PluginLog::open(bea_dir)?;
    let entries = log.recent(limit, plugin_filter, level_filter)?;

    if entries.is_empty() {
        println!("No log entries recorded yet.");
        return Ok(());
    }

    println!(
        "{:<5} {:<22} {:<12} {:<8} {}",
        "ID", "TIMESTAMP", "PLUGIN", "LEVEL", "MESSAGE"
    );
    println!("{}", "-".repeat(80));

    for entry in &entries {
        let msg_display = if entry.message.len() > 60 {
            format!("{}...", &entry.message[..60])
        } else {
            entry.message.clone()
        };
        println!(
            "{:<5} {:<22} {:<12} {:<8} {}",
            entry.id, entry.timestamp, entry.plugin, entry.level, msg_display
        );
    }

    println!("\n{} log entry(ies) shown", entries.len());
    Ok(())
}

fn cmd_version(bea_dir_flag: Option<PathBuf>) {
    println!("bea {}", env!("CARGO_PKG_VERSION"));

    // Show .bea directory if found.
    match resolve_bea_dir(bea_dir_flag) {
        Ok(bea_dir) => {
            println!("  bea_dir: {}", bea_dir.display());
            // Count plugins.
            let count = std::fs::read_dir(bea_dir.join("plugins"))
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path().extension().is_some_and(|ext| ext == "wasm")
                        })
                        .count()
                })
                .unwrap_or(0);
            println!("  plugins: {count}");
        }
        Err(_) => {
            println!("  bea_dir: (not found)");
        }
    }
}

fn cmd_import(
    bea_dir: &std::path::Path,
    plugin: &str,
    file: &std::path::Path,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("file not found: {}", file.display()).into());
    }

    let content = std::fs::read_to_string(file)?;
    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in {}: {e}", file.display()))?;

    let mut db = bea_host::db::PluginDb::new(bea_dir);
    db.ensure_dir()?;

    let (tables, rows) = db.import_plugin(plugin, &data)?;

    if !quiet {
        println!("Imported {rows} rows across {tables} tables into '{plugin}'.");
    }
    Ok(())
}

fn cmd_reset(
    bea_dir: &std::path::Path,
    plugin: &str,
    force: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = bea_dir.join(format!("data/{plugin}.db"));
    if !db_path.exists() {
        return Err(format!("no database found for plugin '{plugin}'").into());
    }

    if !force {
        // In non-interactive mode (e.g., scripts), require --force.
        // Check if stdin is a terminal.
        return Err(
            "this will delete all data for '{plugin}'. Use --force to confirm.".into(),
        );
    }

    let mut db = bea_host::db::PluginDb::new(bea_dir);
    db.ensure_dir()?;

    let tables = db.reset_plugin(plugin)?;

    if !quiet {
        println!("Reset '{plugin}': dropped {tables} table(s).");
    }
    Ok(())
}

fn cmd_config(
    bea_dir: &std::path::Path,
    plugin: &str,
    key: Option<&str>,
    value: Option<&str>,
    delete: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = bea_dir.join(format!("config/{plugin}.toml"));

    // Validate plugin exists.
    let wasm_path = bea_dir.join(format!("plugins/{plugin}.wasm"));
    if !wasm_path.exists() {
        return Err(format!("plugin '{plugin}' not installed").into());
    }

    match (key, value, delete) {
        // bea config <plugin> -- show all
        (None, None, false) => {
            if !config_path.exists() {
                println!("No configuration for '{plugin}'.");
                println!("Set a value: bea config {plugin} <key> <value>");
                return Ok(());
            }
            let content = std::fs::read_to_string(&config_path)?;
            if content.trim().is_empty() {
                println!("No configuration for '{plugin}'.");
                return Ok(());
            }
            let table: toml::Table = content
                .parse()
                .map_err(|e| format!("failed to parse {}: {e}", config_path.display()))?;
            let mut flat = std::collections::BTreeMap::new();
            flatten_config("", &toml::Value::Table(table), &mut flat);
            for (k, v) in &flat {
                println!("{k} = {v}");
            }
            Ok(())
        }
        // bea config <plugin> <key> -- get a specific key
        (Some(k), None, false) => {
            if !config_path.exists() {
                return Err(format!("key '{k}' not found (no config for '{plugin}')").into());
            }
            let content = std::fs::read_to_string(&config_path)?;
            let table: toml::Table = content
                .parse()
                .map_err(|e| format!("failed to parse {}: {e}", config_path.display()))?;
            let mut flat = std::collections::BTreeMap::new();
            flatten_config("", &toml::Value::Table(table), &mut flat);
            match flat.get(k) {
                Some(v) => {
                    println!("{v}");
                    Ok(())
                }
                None => Err(format!("key '{k}' not found in {plugin} config").into()),
            }
        }
        // bea config <plugin> <key> <value> -- set a key
        (Some(k), Some(v), false) => {
            let mut table: toml::Table = if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                content
                    .parse()
                    .map_err(|e| format!("failed to parse {}: {e}", config_path.display()))?
            } else {
                toml::Table::new()
            };

            set_nested_key(&mut table, k, v);

            let content = toml::to_string_pretty(&table)?;
            std::fs::write(&config_path, content)?;
            println!("Set {plugin}.{k} = {v}");
            Ok(())
        }
        // bea config <plugin> <key> --delete -- delete a key
        (Some(k), None, true) => {
            if !config_path.exists() {
                return Err(format!("key '{k}' not found (no config for '{plugin}')").into());
            }
            let content = std::fs::read_to_string(&config_path)?;
            let mut table: toml::Table = content
                .parse()
                .map_err(|e| format!("failed to parse {}: {e}", config_path.display()))?;

            if !delete_nested_key(&mut table, k) {
                return Err(format!("key '{k}' not found in {plugin} config").into());
            }

            let content = toml::to_string_pretty(&table)?;
            std::fs::write(&config_path, content)?;
            println!("Deleted {plugin}.{k}");
            Ok(())
        }
        // Invalid combinations
        (None, Some(_), _) => Err("key is required when setting a value".into()),
        (Some(_), Some(_), true) => {
            Err("cannot use --delete with a value (use --delete without a value)".into())
        }
        (None, None, true) => Err("key is required with --delete".into()),
    }
}

/// Flatten a TOML value into dot-separated key = value strings for display.
fn flatten_config(prefix: &str, value: &toml::Value, out: &mut std::collections::BTreeMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_config(&key, v, out);
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

/// Set a potentially nested key (e.g., "section.key") in a TOML table.
fn set_nested_key(table: &mut toml::Table, key: &str, value: &str) {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    if parts.len() == 1 {
        table.insert(key.to_string(), toml::Value::String(value.to_string()));
    } else {
        let section = parts[0];
        let rest = parts[1];
        let sub = table
            .entry(section.to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if let toml::Value::Table(sub_table) = sub {
            set_nested_key(sub_table, rest, value);
        } else {
            // Overwrite non-table value with a table.
            let mut new_table = toml::Table::new();
            set_nested_key(&mut new_table, rest, value);
            *sub = toml::Value::Table(new_table);
        }
    }
}

/// Delete a potentially nested key. Returns true if the key existed.
fn delete_nested_key(table: &mut toml::Table, key: &str) -> bool {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    if parts.len() == 1 {
        table.remove(key).is_some()
    } else {
        let section = parts[0];
        let rest = parts[1];
        if let Some(toml::Value::Table(sub_table)) = table.get_mut(section) {
            let removed = delete_nested_key(sub_table, rest);
            // Clean up empty parent tables.
            if sub_table.is_empty() {
                table.remove(section);
            }
            removed
        } else {
            false
        }
    }
}

fn cmd_list(
    bea_dir: &std::path::Path,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = PluginRegistry::load(bea_dir, verbose)?;
    let plugins = registry.all_metadata();

    if plugins.is_empty() {
        println!("No plugins installed.");
        println!("Use 'bea install <path.wasm>' to install a plugin.");
        return Ok(());
    }

    let db = bea_host::db::PluginDb::new(bea_dir);

    println!(
        "{:<15} {:<10} {:<8} {:<10}",
        "PLUGIN", "VERSION", "CMDS", "DB SIZE"
    );
    println!("{}", "-".repeat(45));

    for plugin in plugins {
        let db_size = db.db_size(&plugin.name);
        let size_str = match db_size {
            Some(s) if s >= 1024 * 1024 => format!("{:.1}MB", s as f64 / (1024.0 * 1024.0)),
            Some(s) if s >= 1024 => format!("{:.1}KB", s as f64 / 1024.0),
            Some(s) => format!("{s}B"),
            None => "-".into(),
        };

        println!(
            "{:<15} {:<10} {:<8} {:<10}",
            plugin.name,
            plugin.version,
            plugin.commands.len(),
            size_str
        );

        if verbose {
            for cmd in &plugin.commands {
                let args_str: Vec<String> = cmd
                    .args
                    .iter()
                    .map(|a| {
                        if a.required {
                            format!("<{}>", a.name)
                        } else {
                            format!("[{}]", a.name)
                        }
                    })
                    .collect();
                eprintln!(
                    "  {} {}: {}",
                    cmd.name,
                    args_str.join(" "),
                    cmd.description
                );
            }
        }
    }

    Ok(())
}
