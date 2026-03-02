mod cmd;
mod config;
mod rules;
mod sqlite;
mod store;
mod time_helper;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

/// beu - Hardcoded CLI tool for agent workflows.
#[derive(Parser)]
#[command(name = "beu", version, about)]
struct Cli {
    /// Path to the .beu directory (default: .beu in current or ancestor directory).
    #[arg(long, global = true)]
    beu_dir: Option<PathBuf>,

    /// Project ID to operate on (required unless config sets require_project: false).
    #[arg(long, short = 'p', global = true)]
    project: Option<String>,

    /// Show detailed output.
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
    /// Initialize a new .beu project directory.
    Init,

    /// Journal: agent interaction ledger.
    Journal {
        #[command(subcommand)]
        action: JournalAction,
    },

    /// Artifact: deliverable progress tracking.
    Artifact {
        #[command(subcommand)]
        action: ArtifactAction,
    },

    /// Task: work item tracking with sprint view.
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// State: persistent project memory (decisions, blockers, focus, notes).
    State {
        #[command(subcommand)]
        action: StateAction,
    },

    /// Idea: lightweight idea capture.
    Idea {
        #[command(subcommand)]
        action: IdeaAction,
    },

    /// Debug: persistent investigation tracking.
    Debug {
        #[command(subcommand)]
        action: DebugAction,
    },

    /// Test: view available test patterns for agent reference.
    Test {
        #[command(subcommand)]
        action: TestAction,
    },

    /// Save a checkpoint before pausing work.
    Pause {
        /// Checkpoint message describing current state.
        message: Vec<String>,
    },

    /// Resume work: show checkpoint, blockers, and focus items.
    Resume,

    /// Cross-module progress summary.
    Progress,

    /// Validate .beu directory integrity.
    Health {
        /// Attempt to repair issues.
        #[arg(long)]
        repair: bool,
    },

    /// Show project status overview.
    Status,

    /// Compliance gate: verify required docs are tracked and active.
    Check,

    /// Show recent event log entries.
    Events {
        /// Maximum number of events to show (default: 20).
        #[arg(long, short = 'n', default_value = "20")]
        limit: usize,
        /// Filter by module name.
        #[arg(long, short)]
        module: Option<String>,
    },

    /// Export module data as JSON.
    Export {
        /// Module name (journal, artifact, task) or --all.
        module: Option<String>,
        /// Export all module databases.
        #[arg(long)]
        all: bool,
    },

    /// Import data from a JSON file into a module's database.
    Import {
        /// Module name.
        module: String,
        /// Path to JSON file.
        file: PathBuf,
    },

    /// Reset a module's database (drop all tables).
    Reset {
        /// Module name.
        module: String,
        /// Skip confirmation prompt.
        #[arg(long)]
        force: bool,
    },

    /// Cross-project discovery and status.
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Show version and build information.
    Version,

    /// Update agent rule files with the latest embedded content.
    /// Use this after upgrading beu to propagate new agent instructions.
    UpdateRules,
}

#[derive(Subcommand)]
enum TestAction {
    /// Show available test patterns and the test status lifecycle.
    Patterns,
}

#[derive(Subcommand)]
enum ProjectAction {
    /// List all beu projects in the repository.
    List {
        /// Filter by project name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Show status across all projects.
    Status {
        /// Show only a specific project.
        #[arg(long)]
        name: Option<String>,
    },
    /// Show progress across all projects.
    Progress {
        /// Show only a specific project.
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum JournalAction {
    /// Start a new journal session.
    Open,
    /// Record a message in the current session.
    Log {
        /// The message to record.
        message: Vec<String>,
    },
    /// Record a categorized note.
    Note {
        /// Category tag (decision, blocker, observation).
        #[arg(long)]
        tag: String,
        /// The note content.
        message: Vec<String>,
    },
    /// Show a digest of the current session.
    Summary,
    /// Close the current session.
    Close,
}

#[derive(Subcommand)]
enum ArtifactAction {
    /// Register a new artifact to track.
    Add {
        /// Artifact name.
        name: String,
        /// Artifact type (doc, codelab, test, config, spec).
        #[arg(long, rename_all = "verbatim", default_value = "doc")]
        r#type: String,
        /// Optional short description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Update the status of a tracked artifact.
    Status {
        /// Artifact name.
        name: String,
        /// New status (pending, in-progress, review, done).
        status: String,
    },
    /// Show all tracked artifacts.
    List {
        /// Filter by status.
        #[arg(long)]
        filter: Option<String>,
    },
    /// Show details for a specific artifact.
    Show {
        /// Artifact name.
        name: String,
    },
    /// Add or update an artifact's description.
    Describe {
        /// Artifact name.
        name: String,
        /// Description text.
        description: Vec<String>,
    },
    /// Remove a tracked artifact.
    Remove {
        /// Artifact name.
        name: String,
    },
    /// Record a changelog entry for an artifact.
    Changelog {
        /// Artifact name.
        name: String,
        /// Changelog message.
        message: Vec<String>,
    },
    /// Show changelog history for an artifact.
    History {
        /// Artifact name.
        name: String,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// Create a new task.
    Add {
        /// Task title.
        title: Vec<String>,
        /// Priority (low, medium, high, critical).
        #[arg(long, default_value = "medium")]
        priority: String,
        /// Tag for categorization.
        #[arg(long)]
        tag: Option<String>,
    },
    /// List tasks with optional filters.
    List {
        /// Filter by status (open, in-progress, done, blocked).
        #[arg(long)]
        status: Option<String>,
        /// Filter by tag.
        #[arg(long)]
        tag: Option<String>,
        /// Filter by test status (planned, designed, implemented, tested, darklaunched, launched).
        #[arg(long)]
        test_status: Option<String>,
    },
    /// Update a task's status, priority, or tag.
    Update {
        /// Task ID.
        id: i64,
        /// New status.
        #[arg(long)]
        status: Option<String>,
        /// New priority.
        #[arg(long)]
        priority: Option<String>,
        /// New tag.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Mark a task as done.
    Done {
        /// Task ID.
        id: i64,
    },
    /// Show task details.
    Show {
        /// Task ID.
        id: i64,
    },
    /// Update the test status of a task.
    TestStatus {
        /// Task ID.
        id: i64,
        /// New test status (planned, designed, implemented, tested, darklaunched, launched).
        status: String,
    },
    /// Sprint summary of active tasks.
    Sprint,
}

#[derive(Subcommand)]
enum StateAction {
    /// Set a state entry (upserts).
    Set {
        /// Category (decision, blocker, focus, note).
        #[arg(long)]
        category: String,
        /// Key name.
        key: String,
        /// Value.
        value: Vec<String>,
    },
    /// Get a state entry or list all entries.
    Get {
        /// Key name (omit to list all).
        key: Option<String>,
    },
    /// List state entries with optional category filter.
    List {
        /// Filter by category.
        #[arg(long)]
        category: Option<String>,
    },
    /// Remove a state entry.
    Remove {
        /// Key name.
        key: String,
    },
    /// Clear all entries in a category.
    Clear {
        /// Category to clear.
        #[arg(long)]
        category: String,
        /// Confirm the destructive operation.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum IdeaAction {
    /// Capture a new idea or task.
    Add {
        /// Title.
        title: Vec<String>,
        /// Area (api, ui, database, testing, docs, tooling, general).
        #[arg(long, default_value = "general")]
        area: String,
        /// Priority (low, medium, high).
        #[arg(long, default_value = "medium")]
        priority: String,
    },
    /// List todos with optional filters.
    List {
        /// Filter by area.
        #[arg(long)]
        area: Option<String>,
        /// Filter by status (pending, done, archived).
        #[arg(long)]
        status: Option<String>,
    },
    /// Show todo details.
    Show {
        /// Todo ID.
        id: i64,
    },
    /// Mark a todo as done.
    Done {
        /// Todo ID.
        id: i64,
    },
    /// Archive a todo.
    Archive {
        /// Todo ID.
        id: i64,
    },
    /// Add or update a todo's description.
    Describe {
        /// Todo ID.
        id: i64,
        /// Description text.
        description: Vec<String>,
    },
}

#[derive(Subcommand)]
enum DebugAction {
    /// Open a new debug investigation session.
    Open {
        /// Title for the debug session.
        title: Vec<String>,
    },
    /// Log evidence in a debug session.
    Log {
        /// Session slug.
        slug: String,
        /// Evidence message.
        message: Vec<String>,
    },
    /// Record a symptom in a debug session.
    Symptom {
        /// Session slug.
        slug: String,
        /// Symptom description.
        description: Vec<String>,
    },
    /// Record root cause in a debug session.
    Cause {
        /// Session slug.
        slug: String,
        /// Root cause description.
        description: Vec<String>,
    },
    /// Mark a debug session as resolved.
    Resolve {
        /// Session slug.
        slug: String,
    },
    /// List debug sessions.
    List {
        /// Filter by status (investigating, root-cause-found, blocked, resolved).
        #[arg(long)]
        status: Option<String>,
    },
    /// Show debug session timeline.
    Show {
        /// Session slug.
        slug: String,
    },
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
    match cli.command {
        Commands::Init => {
            let root = std::env::current_dir()?;
            cmd::system::cmd_init(&root, cli.quiet)
        }
        Commands::Project { action } => match action {
            ProjectAction::List { name } => cmd::project::cmd_list(name.as_deref()),
            ProjectAction::Status { name } => cmd::project::cmd_status(name.as_deref()),
            ProjectAction::Progress { name } => cmd::project::cmd_progress(name.as_deref()),
        },
        Commands::Version => {
            cmd::system::cmd_version(cli.beu_dir);
            Ok(())
        }
        Commands::UpdateRules => {
            let root = std::env::current_dir()?;
            cmd::system::cmd_update_rules(&root, cli.quiet)
        }
        Commands::Test { action } => {
            let beu_dir = resolve_beu_dir(cli.beu_dir.clone())?;
            let cfg = config::load(&beu_dir)?;
            match action {
                TestAction::Patterns => cmd::testing::cmd_patterns(&cfg),
            }
        }
        ref _cmd => {
            let beu_dir = resolve_beu_dir(cli.beu_dir.clone())?;
            let cfg = config::load(&beu_dir)?;
            let project_id = cfg.resolve_project(cli.project.as_deref())?;
            run_with_project(cli.command, &beu_dir, &cfg, &project_id, cli.quiet)
        }
    }
}

fn run_with_project(
    command: Commands,
    beu_dir: &std::path::Path,
    cfg: &config::BeuConfig,
    project_id: &str,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Journal { action } => {
            cfg.require_module("journal")?;
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let (cmd_name, result) = match action {
                JournalAction::Open => ("open", cmd::journal::cmd_open(&mut store)),
                JournalAction::Log { message } => {
                    let msg = message.join(" ");
                    if msg.is_empty() {
                        return Err("usage: beu journal log <message>".into());
                    }
                    ("log", cmd::journal::cmd_log(&mut store, &msg))
                }
                JournalAction::Note { tag, message } => {
                    let msg = message.join(" ");
                    if msg.is_empty() {
                        return Err("usage: beu journal note --tag <tag> <message>".into());
                    }
                    ("note", cmd::journal::cmd_note(&mut store, &tag, &msg))
                }
                JournalAction::Summary => ("summary", cmd::journal::cmd_summary(&mut store)),
                JournalAction::Close => ("close", cmd::journal::cmd_close(&mut store)),
            };

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "journal", cmd_name, status, duration_ms);
            result
        }
        Commands::Artifact { action } => {
            cfg.require_module("artifact")?;
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let (cmd_name, result) = match action {
                ArtifactAction::Add {
                    name,
                    r#type,
                    description,
                } => (
                    "add",
                    cmd::artifact::cmd_add(&mut store, &name, &r#type, description.as_deref()),
                ),
                ArtifactAction::Status { name, status } => (
                    "status",
                    cmd::artifact::cmd_status(&mut store, &name, &status),
                ),
                ArtifactAction::List { filter } => (
                    "list",
                    cmd::artifact::cmd_list(&mut store, filter.as_deref()),
                ),
                ArtifactAction::Show { name } => {
                    ("show", cmd::artifact::cmd_show(&mut store, &name))
                }
                ArtifactAction::Describe { name, description } => {
                    let desc = description.join(" ");
                    if desc.is_empty() {
                        return Err("usage: beu artifact describe <name> <description>".into());
                    }
                    (
                        "describe",
                        cmd::artifact::cmd_describe(&mut store, &name, &desc),
                    )
                }
                ArtifactAction::Remove { name } => {
                    ("remove", cmd::artifact::cmd_remove(&mut store, &name))
                }
                ArtifactAction::Changelog { name, message } => {
                    let msg = message.join(" ");
                    if msg.is_empty() {
                        return Err("usage: beu artifact changelog <name> <message>".into());
                    }
                    (
                        "changelog",
                        cmd::artifact::cmd_changelog(&mut store, &name, &msg),
                    )
                }
                ArtifactAction::History { name } => {
                    ("history", cmd::artifact::cmd_history(&mut store, &name))
                }
            };

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "artifact", cmd_name, status, duration_ms);
            result
        }
        Commands::Task { action } => {
            cfg.require_module("task")?;
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let (cmd_name, result) = match action {
                TaskAction::Add {
                    title,
                    priority,
                    tag,
                } => {
                    let title_str = title.join(" ");
                    if title_str.is_empty() {
                        return Err("usage: beu task add <title>".into());
                    }
                    (
                        "add",
                        cmd::task::cmd_add(&mut store, &title_str, &priority, tag.as_deref()),
                    )
                }
                TaskAction::List {
                    status,
                    tag,
                    test_status,
                } => (
                    "list",
                    cmd::task::cmd_list(
                        &mut store,
                        status.as_deref(),
                        tag.as_deref(),
                        test_status.as_deref(),
                    ),
                ),

                TaskAction::Update {
                    id,
                    status,
                    priority,
                    tag,
                } => (
                    "update",
                    cmd::task::cmd_update(
                        &mut store,
                        id,
                        status.as_deref(),
                        priority.as_deref(),
                        tag.as_deref(),
                    ),
                ),
                TaskAction::Done { id } => ("done", cmd::task::cmd_done(&mut store, id)),
                TaskAction::Show { id } => ("show", cmd::task::cmd_show(&mut store, id)),
                TaskAction::TestStatus { id, status } => (
                    "test-status",
                    cmd::task::cmd_test_status(&mut store, id, &status),
                ),
                TaskAction::Sprint => ("sprint", cmd::task::cmd_sprint(&mut store)),
            };

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "task", cmd_name, status, duration_ms);
            result
        }
        Commands::State { action } => {
            cfg.require_module("state")?;
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let (cmd_name, result) = match action {
                StateAction::Set {
                    category,
                    key,
                    value,
                } => {
                    let val = value.join(" ");
                    if val.is_empty() {
                        return Err("usage: beu state set --category <C> <key> <value>".into());
                    }
                    (
                        "set",
                        cmd::state::cmd_set(&mut store, &category, &key, &val),
                    )
                }
                StateAction::Get { key } => {
                    ("get", cmd::state::cmd_get(&mut store, key.as_deref()))
                }
                StateAction::List { category } => (
                    "list",
                    cmd::state::cmd_list(&mut store, category.as_deref()),
                ),
                StateAction::Remove { key } => ("remove", cmd::state::cmd_remove(&mut store, &key)),
                StateAction::Clear { category, force } => {
                    ("clear", cmd::state::cmd_clear(&mut store, &category, force))
                }
            };

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "state", cmd_name, status, duration_ms);
            result
        }
        Commands::Idea { action } => {
            cfg.require_module("idea")?;
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let (cmd_name, result) = match action {
                IdeaAction::Add {
                    title,
                    area,
                    priority,
                } => {
                    let title_str = title.join(" ");
                    if title_str.is_empty() {
                        return Err("usage: beu idea add <title>".into());
                    }
                    (
                        "add",
                        cmd::idea::cmd_add(&mut store, &title_str, &area, &priority),
                    )
                }
                IdeaAction::List { area, status } => (
                    "list",
                    cmd::idea::cmd_list(&mut store, area.as_deref(), status.as_deref()),
                ),
                IdeaAction::Show { id } => ("show", cmd::idea::cmd_show(&mut store, id)),
                IdeaAction::Done { id } => ("done", cmd::idea::cmd_done(&mut store, id)),
                IdeaAction::Archive { id } => ("archive", cmd::idea::cmd_archive(&mut store, id)),
                IdeaAction::Describe { id, description } => {
                    let desc = description.join(" ");
                    if desc.is_empty() {
                        return Err("usage: beu idea describe <id> <description>".into());
                    }
                    ("describe", cmd::idea::cmd_describe(&mut store, id, &desc))
                }
            };

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "idea", cmd_name, status, duration_ms);
            result
        }
        Commands::Debug { action } => {
            cfg.require_module("debug")?;
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let (cmd_name, result) = match action {
                DebugAction::Open { title } => {
                    let title_str = title.join(" ");
                    if title_str.is_empty() {
                        return Err("usage: beu debug open <title>".into());
                    }
                    ("open", cmd::debug::cmd_open(&mut store, &title_str))
                }
                DebugAction::Log { slug, message } => {
                    let msg = message.join(" ");
                    if msg.is_empty() {
                        return Err("usage: beu debug log <slug> <message>".into());
                    }
                    ("log", cmd::debug::cmd_log(&mut store, &slug, &msg))
                }
                DebugAction::Symptom { slug, description } => {
                    let desc = description.join(" ");
                    if desc.is_empty() {
                        return Err("usage: beu debug symptom <slug> <description>".into());
                    }
                    ("symptom", cmd::debug::cmd_symptom(&mut store, &slug, &desc))
                }
                DebugAction::Cause { slug, description } => {
                    let desc = description.join(" ");
                    if desc.is_empty() {
                        return Err("usage: beu debug cause <slug> <description>".into());
                    }
                    ("cause", cmd::debug::cmd_cause(&mut store, &slug, &desc))
                }
                DebugAction::Resolve { slug } => {
                    ("resolve", cmd::debug::cmd_resolve(&mut store, &slug))
                }
                DebugAction::List { status } => {
                    ("list", cmd::debug::cmd_list(&mut store, status.as_deref()))
                }
                DebugAction::Show { slug } => ("show", cmd::debug::cmd_show(&mut store, &slug)),
            };

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "debug", cmd_name, status, duration_ms);
            result
        }
        Commands::Pause { message } => {
            cfg.require_module("state")?;
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let msg = message.join(" ");
            let result =
                cmd::system::cmd_pause(&mut store, if msg.is_empty() { None } else { Some(&msg) });

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "system", "pause", status, duration_ms);
            result
        }
        Commands::Resume => {
            cfg.require_module("state")?;
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let result = cmd::system::cmd_resume(&mut store);

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "system", "resume", status, duration_ms);
            result
        }
        Commands::Progress => {
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let result = cmd::system::cmd_progress(&mut store, cfg);

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "system", "progress", status, duration_ms);
            result
        }
        Commands::Health { repair } => {
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;

            let start = std::time::Instant::now();
            let result = cmd::system::cmd_health(&mut store, repair);

            let duration_ms = start.elapsed().as_millis() as i64;
            let status = if result.is_ok() { "ok" } else { "error" };
            cmd::system::log_event(&mut store, "system", "health", status, duration_ms);
            result
        }
        Commands::Status => {
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;
            cmd::system::cmd_status(&mut store, cfg)
        }
        Commands::Check => {
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;
            cmd::system::cmd_check(&mut store, cfg)
        }
        Commands::Events { limit, module } => {
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;
            cmd::system::cmd_events(&mut store, limit, module.as_deref())
        }
        Commands::Export { module, all } => {
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;
            cmd::system::cmd_export(&mut store, module.as_deref(), all)
        }
        Commands::Import { module, file } => {
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;
            cmd::system::cmd_import(&mut store, &module, &file, quiet)
        }
        Commands::Reset { module, force } => {
            let mut store = sqlite::SqliteStore::open(beu_dir, project_id)?;
            cmd::system::cmd_reset(&mut store, &module, force, quiet)
        }
        // Init, Project, Version, Test are handled in run() before reaching here.
        Commands::Init
        | Commands::Project { .. }
        | Commands::Version
        | Commands::UpdateRules
        | Commands::Test { .. } => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// .beu directory resolution
// ---------------------------------------------------------------------------

pub fn resolve_beu_dir(explicit: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(dir) = explicit {
        if !dir.exists() {
            return Err(format!(".beu directory not found at {}", dir.display()).into());
        }
        return Ok(dir);
    }

    let mut current = std::env::current_dir()?;
    loop {
        let candidate = current.join(".beu");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !current.pop() {
            break;
        }
    }

    Err("no .beu directory found (run `beu init` first)".into())
}
