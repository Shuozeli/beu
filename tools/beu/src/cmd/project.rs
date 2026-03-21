//! Cross-project discovery and read-only querying.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config;
use crate::sqlite::SqliteStore;
use crate::store::{ArtifactStore, DebugStore, IdeaStore, StateStore, TaskStore};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A discovered beu subproject within the repository.
pub struct DiscoveredProject {
    /// Relative path from the git root (e.g., "tools/beu", "frontend").
    pub name: String,
    /// Absolute path to the .beu directory.
    pub beu_dir: PathBuf,
}

// ---------------------------------------------------------------------------
// Git root discovery
// ---------------------------------------------------------------------------

/// Walk up from `start` looking for a `.git/` directory or file.
/// Returns the directory containing `.git/`, or an error.
pub fn find_git_root(start: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut current = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()?.join(start)
    };
    loop {
        if current.join(".git").exists() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }
    Err("not inside a git repository (no .git/ found)".into())
}

// ---------------------------------------------------------------------------
// Project discovery
// ---------------------------------------------------------------------------

/// Directories to skip during traversal (performance + correctness).
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "vendor",
    "__pycache__",
    ".venv",
    "dist",
    "build",
];

/// Discover all `.beu/` directories under `git_root`.
/// Returns them sorted by name (relative path).
pub fn discover_projects(
    git_root: &Path,
) -> Result<Vec<DiscoveredProject>, Box<dyn std::error::Error>> {
    let mut projects = Vec::new();

    let walker = WalkDir::new(git_root)
        .max_depth(6)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == ".beu" {
                        return false;
                    }
                    if SKIP_DIRS.contains(&name) {
                        return false;
                    }
                }
            }
            true
        });

    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_dir() {
            let beu_candidate = entry.path().join(".beu");
            if beu_candidate.is_dir() && beu_candidate.join("data/beu.db").exists() {
                let rel_path = entry.path().strip_prefix(git_root).unwrap_or(entry.path());
                let name = if rel_path.as_os_str().is_empty() {
                    ".".to_string()
                } else {
                    rel_path.to_string_lossy().to_string()
                };
                projects.push(DiscoveredProject {
                    name,
                    beu_dir: beu_candidate,
                });
            }
        }
    }

    projects.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(projects)
}

// ---------------------------------------------------------------------------
// Summary collection (read-only)
// ---------------------------------------------------------------------------

fn collect_summary_line(project: &DiscoveredProject) -> Vec<String> {
    let config = config::load(&project.beu_dir).unwrap_or_default();
    let mut store = match SqliteStore::open_readonly(&project.beu_dir, "default") {
        Ok(s) => s,
        Err(_) => return vec!["  (error opening database)".to_string()],
    };

    let mut lines = Vec::new();

    if config.is_module_enabled("state") {
        if let Ok(Some(msg)) = store.get_checkpoint() {
            lines.push(format!("  Checkpoint: {msg}"));
        }
        if let Ok(count) = store.count_by_category("blocker") {
            if count > 0 {
                lines.push(format!("  Blockers:   {count}"));
            }
        }
    }

    if config.is_module_enabled("task") {
        if let Ok(counts) = store.count_tasks_by_status() {
            if !counts.is_empty() {
                let parts: Vec<String> = counts.iter().map(|(s, c)| format!("{s}: {c}")).collect();
                lines.push(format!("  Tasks:      {}", parts.join(", ")));
            }
        }
    }

    if config.is_module_enabled("artifact") {
        if let Ok(artifacts) = store.list_artifacts(None) {
            if !artifacts.is_empty() {
                let mut counts: std::collections::BTreeMap<&str, usize> =
                    std::collections::BTreeMap::new();
                for a in &artifacts {
                    *counts.entry(&a.status).or_insert(0) += 1;
                }
                let parts: Vec<String> = counts.iter().map(|(s, c)| format!("{s}: {c}")).collect();
                lines.push(format!("  Artifacts:  {}", parts.join(", ")));
            }
        }
    }

    if config.is_module_enabled("idea") {
        if let Ok(counts) = store.count_ideas_by_status() {
            if !counts.is_empty() {
                let parts: Vec<String> = counts.iter().map(|(s, c)| format!("{s}: {c}")).collect();
                lines.push(format!("  Ideas:      {}", parts.join(", ")));
            }
        }
    }

    if config.is_module_enabled("debug") {
        if let Ok(count) = store.count_active() {
            if count > 0 {
                lines.push(format!("  Debug:      {count} active"));
            }
        }
    }

    if lines.is_empty() {
        lines.push("  (no data)".to_string());
    }

    lines
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

pub fn cmd_list(name_filter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let git_root = find_git_root(&cwd)?;
    let projects = discover_projects(&git_root)?;

    if projects.is_empty() {
        println!("No beu projects found in this repository.");
        return Ok(());
    }

    let filtered: Vec<&DiscoveredProject> = match name_filter {
        Some(name) => projects.iter().filter(|p| p.name == name).collect(),
        None => projects.iter().collect(),
    };

    if filtered.is_empty() {
        println!("No matching project found.");
        return Ok(());
    }

    for p in &filtered {
        let config = config::load(&p.beu_dir).unwrap_or_default();
        let modules = config.enabled_modules().join(", ");
        println!("  {} ({})", p.name, modules);
    }
    println!("\n{} project(s) found.", filtered.len());

    Ok(())
}

pub fn cmd_status(name_filter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let git_root = find_git_root(&cwd)?;
    let projects = discover_projects(&git_root)?;

    if projects.is_empty() {
        println!("No beu projects found in this repository.");
        return Ok(());
    }

    let filtered: Vec<&DiscoveredProject> = match name_filter {
        Some(name) => projects.iter().filter(|p| p.name == name).collect(),
        None => projects.iter().collect(),
    };

    if filtered.is_empty() {
        return Err(format!("project '{}' not found", name_filter.unwrap_or("?")).into());
    }

    for p in &filtered {
        println!("--- {} ---", p.name);
        let config = config::load(&p.beu_dir).unwrap_or_default();
        println!("  modules: {}", config.enabled_modules().join(", "));
        match SqliteStore::open_readonly(&p.beu_dir, "default") {
            Ok(store) => {
                if let Some(size) = store.db_size() {
                    println!("  data:    {}", super::system::format_byte_size(size));
                }
            }
            Err(e) => {
                println!("  (error opening: {e})");
            }
        }
        println!();
    }

    println!("{} project(s).", filtered.len());
    Ok(())
}

pub fn cmd_progress(name_filter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let git_root = find_git_root(&cwd)?;
    let projects = discover_projects(&git_root)?;

    if projects.is_empty() {
        println!("No beu projects found in this repository.");
        return Ok(());
    }

    let filtered: Vec<&DiscoveredProject> = match name_filter {
        Some(name) => projects.iter().filter(|p| p.name == name).collect(),
        None => projects.iter().collect(),
    };

    if filtered.is_empty() {
        return Err(format!("project '{}' not found", name_filter.unwrap_or("?")).into());
    }

    println!("=== Repo Projects ===\n");

    for p in &filtered {
        println!("--- {} ---", p.name);
        for line in collect_summary_line(p) {
            println!("{line}");
        }
        println!();
    }

    println!("{} project(s).", filtered.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_fake_repo() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".git")).unwrap();

        for subdir in &["alpha", "beta/nested"] {
            let project_dir = root.join(subdir);
            std::fs::create_dir_all(&project_dir).unwrap();
            let beu_dir = project_dir.join(".beu");
            SqliteStore::open(&beu_dir, "default").unwrap();
            config::save(&beu_dir, &config::BeuConfig::default()).unwrap();
        }

        tmp
    }

    #[test]
    fn find_git_root_from_subdirectory() {
        let tmp = setup_fake_repo();
        let deep = tmp.path().join("alpha/src/lib");
        std::fs::create_dir_all(&deep).unwrap();
        let root = find_git_root(&deep).unwrap();
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn find_git_root_not_found() {
        let tmp = TempDir::new().unwrap();
        let err = find_git_root(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("not inside a git repository"));
    }

    #[test]
    fn discover_finds_all_projects() {
        let tmp = setup_fake_repo();
        let projects = discover_projects(tmp.path()).unwrap();
        assert_eq!(projects.len(), 2);
        let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta/nested"));
    }

    #[test]
    fn discover_sorted_by_name() {
        let tmp = setup_fake_repo();
        let projects = discover_projects(tmp.path()).unwrap();
        assert!(projects[0].name < projects[1].name);
    }

    #[test]
    fn discover_empty_repo() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let projects = discover_projects(tmp.path()).unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn discover_root_project() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let beu_dir = tmp.path().join(".beu");
        SqliteStore::open(&beu_dir, "default").unwrap();
        config::save(&beu_dir, &config::BeuConfig::default()).unwrap();

        let projects = discover_projects(tmp.path()).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, ".");
    }

    #[test]
    fn discover_skips_node_modules() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();

        // Put a .beu inside node_modules -- should be skipped.
        let nm_beu = tmp.path().join("node_modules/pkg/.beu");
        SqliteStore::open(&nm_beu, "default").unwrap();

        // Put a real .beu at top level.
        let real_beu = tmp.path().join("app/.beu");
        std::fs::create_dir_all(tmp.path().join("app")).unwrap();
        SqliteStore::open(&real_beu, "default").unwrap();
        config::save(&real_beu, &config::BeuConfig::default()).unwrap();

        let projects = discover_projects(tmp.path()).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "app");
    }

    #[test]
    fn collect_summary_empty_project() {
        let tmp = setup_fake_repo();
        let project = DiscoveredProject {
            name: "alpha".to_string(),
            beu_dir: tmp.path().join("alpha/.beu"),
        };
        let lines = collect_summary_line(&project);
        assert_eq!(lines, vec!["  (no data)"]);
    }

    #[test]
    fn collect_summary_with_tasks() {
        let tmp = setup_fake_repo();
        let beu_dir = tmp.path().join("alpha/.beu");
        {
            let mut store = SqliteStore::open(&beu_dir, "default").unwrap();
            store.add_task("test task", "medium", None).unwrap();
        }

        let project = DiscoveredProject {
            name: "alpha".to_string(),
            beu_dir,
        };
        let lines = collect_summary_line(&project);
        assert!(lines.iter().any(|l| l.contains("Tasks:")));
    }
}
