use std::path::Path;
use std::process::Command;

/// The GitHub repo that provides skill files via `npx skills`.
const SKILLS_REPO: &str = "Shuozeli/beu";

/// Download and install skill rule files into the agent rule directories under `root`.
///
/// Shells out to: `npx skills add <SKILLS_REPO> --all [--copy]`
///
/// The `skills` CLI clones the repo, discovers SKILL.md files, and installs
/// them into agent rule directories (.claude/rules/, .gemini/rules/, .agent/rules/).
///
/// Returns a list of output lines from the skills CLI.
pub fn install_skills(
    root: &Path,
    _force: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut cmd = Command::new("npx");
    cmd.arg("--yes")
        .arg("skills")
        .arg("add")
        .arg(SKILLS_REPO)
        .arg("--all")
        .arg("--copy")
        .current_dir(root);

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
