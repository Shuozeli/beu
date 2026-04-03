#![allow(dead_code)]

use std::process::Command;
use tempfile::TempDir;

pub fn beu_cmd(beu_dir: &str) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_beu"));
    cmd.arg("--beu-dir").arg(beu_dir);
    cmd
}

pub fn setup() -> TempDir {
    let tmp = TempDir::new().unwrap();
    // Initialize beu project.
    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    tmp
}

pub fn beu_dir_path(tmp: &TempDir) -> String {
    tmp.path().join(".beu").to_string_lossy().to_string()
}

/// Append a required_docs section to config.yml.
pub fn set_required_docs(tmp: &TempDir, docs: &[(&str, &str)]) {
    let config_path = tmp.path().join(".beu/config.yml");
    let mut content = std::fs::read_to_string(&config_path).unwrap();
    content.push_str("required_docs:\n");
    for (name, doc_type) in docs {
        content.push_str(&format!("  - name: {name}\n    type: {doc_type}\n"));
    }
    std::fs::write(&config_path, content).unwrap();
}

/// Write a config.yml that disables the given module.
pub fn disable_module(tmp: &TempDir, module: &str) {
    let config_path = tmp.path().join(".beu/config.yml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let updated = content.replace(&format!("{module}: true"), &format!("{module}: false"));
    std::fs::write(&config_path, updated).unwrap();
}
