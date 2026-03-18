use std::path::Path;
use std::process::Command;

/// The GitHub repo that provides skill files via `npx skills`.
const SKILLS_REPO: &str = "Shuozeli/beu";

/// Default agents to install skills for.
/// Only Claude Code and the generic .agents directory.
const DEFAULT_AGENTS: &[&str] = &["Claude Code", "Agents"];

/// Download and install skill rule files into agent directories under `root`.
///
/// By default, installs only for Claude Code and .agents/.
/// Pass `agents` to override with specific agent names, or `None` for defaults.
/// Pass `all_agents: true` to install for all known agents (42+).
pub fn install_skills(
    root: &Path,
    all_agents: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut cmd = Command::new("npx");
    cmd.arg("--yes")
        .arg("skills")
        .arg("add")
        .arg(SKILLS_REPO)
        .arg("--skill").arg("*")
        .arg("--copy")
        .arg("-y")
        .current_dir(root);

    if all_agents {
        cmd.arg("--agent").arg("*");
    } else {
        for agent in DEFAULT_AGENTS {
            cmd.arg("--agent").arg(agent);
        }
    }

    let output = cmd.output().map_err(|e| {
        format!("failed to run npx: {e}\n  make sure Node.js and npx are installed and in PATH")
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("npx skills add {SKILLS_REPO} failed:\n{stderr}").into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let written: Vec<String> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();

    Ok(written)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirm the SKILLS_REPO constant points to the correct repo.
    #[test]
    fn skills_repo_is_correct() {
        assert_eq!(SKILLS_REPO, "Shuozeli/beu");
    }

    #[test]
    fn default_agents_includes_claude() {
        assert!(DEFAULT_AGENTS.contains(&"Claude Code"));
    }

    /// Verify install_skills writes the expected rule files via npx skills.
    /// Requires Node.js + npx in PATH and network access to GitHub.
    #[test]
    #[ignore = "requires npx and network access"]
    fn install_skills_writes_rule_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let _written = install_skills(tmp.path(), false).unwrap();

        assert!(
            tmp.path().join(".claude/rules/beu.md").exists()
                || tmp.path().join(".claude/skills/beu/SKILL.md").exists()
        );
    }
}
