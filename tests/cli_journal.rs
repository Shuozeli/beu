mod common;

use common::{beu_cmd, beu_dir_path, setup};

#[test]
fn cli_journal_open_and_close() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("opened"));

    let output = beu_cmd(&dir).args(["journal", "close"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("closed"));
}

#[test]
fn cli_journal_log_and_summary() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();

    let output = beu_cmd(&dir)
        .args(["journal", "log", "hello", "world"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Logged: hello world"));

    let output = beu_cmd(&dir).args(["journal", "summary"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hello world"));
}

#[test]
fn cli_journal_note() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();

    let output = beu_cmd(&dir)
        .args(["journal", "note", "--tag", "decision", "use", "sqlite"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[decision]"));

    let output = beu_cmd(&dir).args(["journal", "summary"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[decision]"));
    assert!(stdout.contains("use sqlite"));
}

#[test]
fn cli_journal_log_fails_without_session() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["journal", "log", "hello"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no open session"));
}

#[test]
fn cli_journal_close_then_log_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir).args(["journal", "close"]).output().unwrap();

    let output = beu_cmd(&dir)
        .args(["journal", "log", "should", "fail"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no open session"));
}

#[test]
fn cli_journal_summary_no_entries() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();

    let output = beu_cmd(&dir).args(["journal", "summary"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Session:"));
    assert!(stdout.contains("no entries yet"));
}

#[test]
fn cli_journal_summary_fails_without_session() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["journal", "summary"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no open session"));
}

#[test]
fn cli_journal_multiple_entries_with_mixed_tags() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "plain", "entry"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["journal", "note", "--tag", "decision", "chose", "sqlite"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["journal", "note", "--tag", "blocker", "need", "approval"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "second", "plain"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["journal", "summary"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("plain entry"));
    assert!(stdout.contains("[decision]"));
    assert!(stdout.contains("chose sqlite"));
    assert!(stdout.contains("[blocker]"));
    assert!(stdout.contains("need approval"));
    assert!(stdout.contains("second plain"));
}

#[test]
fn cli_journal_close_and_reopen() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "first", "session"])
        .output()
        .unwrap();
    beu_cmd(&dir).args(["journal", "close"]).output().unwrap();

    // Open a new session.
    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "second", "session"])
        .output()
        .unwrap();

    // Summary should show the new session only.
    let output = beu_cmd(&dir).args(["journal", "summary"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("second session"));
    // First session's entries should not appear.
    assert!(!stdout.contains("first session"));
}

#[test]
fn cli_journal_note_fails_without_session() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["journal", "note", "--tag", "decision", "test"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no open session"));
}

#[test]
fn cli_journal_close_fails_without_session() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["journal", "close"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no open session"));
}
