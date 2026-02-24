use bea_sdk::{
    ArgDef, CommandDef, CommandInput, CommandOutput, DbExecRequest, DbExecResponse,
    PluginMetadata,
};
use extism_pdk::*;

// ---------------------------------------------------------------------------
// Host function imports
// ---------------------------------------------------------------------------

#[host_fn]
extern "ExtismHost" {
    fn host_db_exec(request_json: String) -> String;
    fn host_log(level: String, message: String) -> String;
    fn host_time() -> String;
}

// ---------------------------------------------------------------------------
// Wasm exports: metadata() and run_command()
// ---------------------------------------------------------------------------

#[plugin_fn]
pub fn metadata() -> FnResult<String> {
    let meta = PluginMetadata {
        name: "track".into(),
        version: "0.1.0".into(),
        description: "Artifact progress tracking. Registers and monitors project deliverables."
            .into(),
        commands: vec![
            CommandDef {
                name: "add".into(),
                description: "Register a new artifact to track.".into(),
                args: vec![
                    ArgDef {
                        name: "name".into(),
                        description: "Artifact name (e.g. 'architecture', 'design-doc').".into(),
                        required: true,
                    },
                    ArgDef {
                        name: "--type".into(),
                        description:
                            "Artifact type: doc, codelab, test, config, spec (default: doc)."
                                .into(),
                        required: false,
                    },
                ],
            },
            CommandDef {
                name: "status".into(),
                description: "Update the status of a tracked artifact.".into(),
                args: vec![
                    ArgDef {
                        name: "name".into(),
                        description: "Artifact name.".into(),
                        required: true,
                    },
                    ArgDef {
                        name: "status".into(),
                        description:
                            "New status: pending, in-progress, review, done.".into(),
                        required: true,
                    },
                ],
            },
            CommandDef {
                name: "list".into(),
                description: "Show all tracked artifacts and their status.".into(),
                args: vec![ArgDef {
                    name: "--filter".into(),
                    description: "Filter by status (e.g. --filter pending).".into(),
                    required: false,
                }],
            },
            CommandDef {
                name: "show".into(),
                description: "Show details for a specific artifact.".into(),
                args: vec![ArgDef {
                    name: "name".into(),
                    description: "Artifact name.".into(),
                    required: true,
                }],
            },
            CommandDef {
                name: "remove".into(),
                description: "Remove a tracked artifact.".into(),
                args: vec![ArgDef {
                    name: "name".into(),
                    description: "Artifact name to remove.".into(),
                    required: true,
                }],
            },
        ],
    };
    let json = serde_json::to_string(&meta).map_err(|e| Error::msg(e.to_string()))?;
    Ok(json)
}

#[plugin_fn]
pub fn run_command(input: String) -> FnResult<String> {
    let cmd: CommandInput =
        serde_json::from_str(&input).map_err(|e| Error::msg(format!("invalid input: {e}")))?;

    ensure_schema()?;

    let result = match cmd.command.as_str() {
        "add" => handle_add(&cmd.args)?,
        "status" => handle_status(&cmd.args)?,
        "list" => handle_list(&cmd.args)?,
        "show" => handle_show(&cmd.args)?,
        "remove" => handle_remove(&cmd.args)?,
        other => CommandOutput::error(format!("unknown command: {other}")),
    };

    let json = serde_json::to_string(&result).map_err(|e| Error::msg(e.to_string()))?;
    Ok(json)
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

fn ensure_schema() -> Result<(), Error> {
    db_exec(
        "CREATE TABLE IF NOT EXISTS artifacts (\
            name TEXT PRIMARY KEY, \
            artifact_type TEXT NOT NULL DEFAULT 'doc', \
            status TEXT NOT NULL DEFAULT 'pending', \
            created_at TEXT NOT NULL, \
            updated_at TEXT NOT NULL\
        )",
        vec![],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

fn handle_add(args: &[String]) -> Result<CommandOutput, Error> {
    if args.is_empty() {
        return Ok(CommandOutput::error("usage: bea track add <name> [--type <type>]"));
    }

    let (name, artifact_type) = parse_add_args(args);
    let now = now_iso()?;

    // Check if already exists.
    let existing = db_exec(
        "SELECT name FROM artifacts WHERE name = ?",
        vec![json_str(&name)],
    )?;
    if !existing.rows.is_empty() {
        return Ok(CommandOutput::error(format!(
            "artifact '{name}' already exists"
        )));
    }

    db_exec(
        "INSERT INTO artifacts (name, artifact_type, status, created_at, updated_at) VALUES (?, ?, 'pending', ?, ?)",
        vec![json_str(&name), json_str(&artifact_type), json_str(&now), json_str(&now)],
    )?;

    log_info(&format!("added artifact: {name} (type: {artifact_type})"))?;

    Ok(CommandOutput::ok_with_data(
        format!("Tracking '{name}' ({artifact_type}) - status: pending"),
        serde_json::json!({
            "name": name,
            "type": artifact_type,
            "status": "pending",
        }),
    ))
}

fn handle_status(args: &[String]) -> Result<CommandOutput, Error> {
    if args.len() < 2 {
        return Ok(CommandOutput::error(
            "usage: bea track status <name> <pending|in-progress|review|done>",
        ));
    }

    let name = &args[0];
    let new_status = &args[1];

    let valid = ["pending", "in-progress", "review", "done"];
    if !valid.contains(&new_status.as_str()) {
        return Ok(CommandOutput::error(format!(
            "invalid status '{new_status}' (valid: {})",
            valid.join(", ")
        )));
    }

    // Check exists.
    let existing = db_exec(
        "SELECT status FROM artifacts WHERE name = ?",
        vec![json_str(name)],
    )?;
    if existing.rows.is_empty() {
        return Ok(CommandOutput::error(format!(
            "artifact '{name}' not found"
        )));
    }

    let old_status = existing.rows[0]
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let now = now_iso()?;
    db_exec(
        "UPDATE artifacts SET status = ?, updated_at = ? WHERE name = ?",
        vec![json_str(new_status), json_str(&now), json_str(name)],
    )?;

    log_info(&format!("{name}: {old_status} -> {new_status}"))?;

    Ok(CommandOutput::ok(format!(
        "'{name}': {old_status} -> {new_status}"
    )))
}

fn handle_list(args: &[String]) -> Result<CommandOutput, Error> {
    let filter = parse_filter_arg(args);

    let resp = if let Some(status) = &filter {
        db_exec(
            "SELECT name, artifact_type, status, updated_at FROM artifacts WHERE status = ? ORDER BY name",
            vec![json_str(status)],
        )?
    } else {
        db_exec(
            "SELECT name, artifact_type, status, updated_at FROM artifacts ORDER BY name",
            vec![],
        )?
    };

    if resp.rows.is_empty() {
        let msg = if let Some(s) = &filter {
            format!("No artifacts with status '{s}'.")
        } else {
            "No artifacts tracked yet. Use 'bea track add <name>' to start.".into()
        };
        return Ok(CommandOutput::ok(msg));
    }

    let mut output = String::new();
    let mut items = Vec::new();
    for row in &resp.rows {
        let name = row.first().and_then(|v| v.as_str()).unwrap_or("?");
        let atype = row.get(1).and_then(|v| v.as_str()).unwrap_or("?");
        let status = row.get(2).and_then(|v| v.as_str()).unwrap_or("?");
        let updated = row.get(3).and_then(|v| v.as_str()).unwrap_or("?");

        output.push_str(&format!("  [{status}] {name} ({atype}) - updated {updated}\n"));
        items.push(serde_json::json!({
            "name": name,
            "type": atype,
            "status": status,
            "updated_at": updated,
        }));
    }

    Ok(CommandOutput::ok_with_data(
        output,
        serde_json::json!({
            "count": items.len(),
            "artifacts": items,
        }),
    ))
}

fn handle_show(args: &[String]) -> Result<CommandOutput, Error> {
    if args.is_empty() {
        return Ok(CommandOutput::error("usage: bea track show <name>"));
    }

    let name = &args[0];
    let resp = db_exec(
        "SELECT name, artifact_type, status, created_at, updated_at FROM artifacts WHERE name = ?",
        vec![json_str(name)],
    )?;

    if resp.rows.is_empty() {
        return Ok(CommandOutput::error(format!(
            "artifact '{name}' not found"
        )));
    }

    let row = &resp.rows[0];
    let atype = row.get(1).and_then(|v| v.as_str()).unwrap_or("?");
    let status = row.get(2).and_then(|v| v.as_str()).unwrap_or("?");
    let created = row.get(3).and_then(|v| v.as_str()).unwrap_or("?");
    let updated = row.get(4).and_then(|v| v.as_str()).unwrap_or("?");

    let msg = format!(
        "Artifact: {name}\nType: {atype}\nStatus: {status}\nCreated: {created}\nUpdated: {updated}"
    );

    Ok(CommandOutput::ok_with_data(
        msg,
        serde_json::json!({
            "name": name,
            "type": atype,
            "status": status,
            "created_at": created,
            "updated_at": updated,
        }),
    ))
}

fn handle_remove(args: &[String]) -> Result<CommandOutput, Error> {
    if args.is_empty() {
        return Ok(CommandOutput::error("usage: bea track remove <name>"));
    }

    let name = &args[0];

    let existing = db_exec(
        "SELECT name FROM artifacts WHERE name = ?",
        vec![json_str(name)],
    )?;
    if existing.rows.is_empty() {
        return Ok(CommandOutput::error(format!(
            "artifact '{name}' not found"
        )));
    }

    db_exec(
        "DELETE FROM artifacts WHERE name = ?",
        vec![json_str(name)],
    )?;

    log_info(&format!("removed artifact: {name}"))?;

    Ok(CommandOutput::ok(format!("Removed artifact '{name}'.")))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_add_args(args: &[String]) -> (String, String) {
    let mut name = String::new();
    let mut artifact_type = "doc".to_string();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--type" && i + 1 < args.len() {
            artifact_type = args[i + 1].clone();
            i += 2;
        } else if name.is_empty() {
            name = args[i].clone();
            i += 1;
        } else {
            i += 1;
        }
    }
    (name, artifact_type)
}

fn parse_filter_arg(args: &[String]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--filter" && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        i += 1;
    }
    None
}

fn db_exec(sql: &str, params: Vec<serde_json::Value>) -> Result<DbExecResponse, Error> {
    let request = DbExecRequest {
        sql: sql.to_string(),
        params,
    };
    let request_json = serde_json::to_string(&request)?;
    let response_json = unsafe { host_db_exec(request_json)? };
    let response: DbExecResponse = serde_json::from_str(&response_json)
        .map_err(|e| Error::msg(format!("host_db_exec response parse error: {e}")))?;
    if let Some(err) = &response.error {
        return Err(Error::msg(format!("SQL error: {err}")));
    }
    Ok(response)
}

fn log_info(msg: &str) -> Result<(), Error> {
    unsafe {
        host_log("info".to_string(), msg.to_string())?;
    }
    Ok(())
}

fn json_str(s: &str) -> serde_json::Value {
    serde_json::Value::String(s.to_string())
}

fn now_iso() -> Result<String, Error> {
    let ts = unsafe { host_time()? };
    Ok(ts)
}
