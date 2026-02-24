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
        name: "journal".into(),
        version: "0.1.0".into(),
        description: "Agent interaction ledger. Records decisions, blockers, and reasoning.".into(),
        commands: vec![
            CommandDef {
                name: "open".into(),
                description: "Start a new journal session.".into(),
                args: vec![],
            },
            CommandDef {
                name: "log".into(),
                description: "Record a message in the current session.".into(),
                args: vec![ArgDef {
                    name: "message".into(),
                    description: "The message to record.".into(),
                    required: true,
                }],
            },
            CommandDef {
                name: "note".into(),
                description: "Record a categorized note (decision, blocker, observation).".into(),
                args: vec![
                    ArgDef {
                        name: "--tag".into(),
                        description: "Category tag: decision, blocker, or observation.".into(),
                        required: true,
                    },
                    ArgDef {
                        name: "message".into(),
                        description: "The note content.".into(),
                        required: true,
                    },
                ],
            },
            CommandDef {
                name: "summary".into(),
                description: "Show a digest of the current session.".into(),
                args: vec![],
            },
            CommandDef {
                name: "close".into(),
                description: "Close the current session.".into(),
                args: vec![],
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
        "open" => handle_open()?,
        "log" => handle_log(&cmd.args)?,
        "note" => handle_note(&cmd.args)?,
        "summary" => handle_summary()?,
        "close" => handle_close()?,
        other => CommandOutput::error(format!("unknown command: {other}")),
    };

    let json = serde_json::to_string(&result).map_err(|e| Error::msg(e.to_string()))?;
    Ok(json)
}

// ---------------------------------------------------------------------------
// Schema initialization
// ---------------------------------------------------------------------------

fn ensure_schema() -> Result<(), Error> {
    db_exec(
        "CREATE TABLE IF NOT EXISTS sessions (\
            id TEXT PRIMARY KEY, \
            started_at TEXT NOT NULL, \
            closed_at TEXT, \
            status TEXT NOT NULL DEFAULT 'open'\
        )",
        vec![],
    )?;
    db_exec(
        "CREATE TABLE IF NOT EXISTS entries (\
            id TEXT PRIMARY KEY, \
            session_id TEXT NOT NULL, \
            created_at TEXT NOT NULL, \
            message TEXT NOT NULL, \
            tag TEXT, \
            FOREIGN KEY (session_id) REFERENCES sessions(id)\
        )",
        vec![],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

fn handle_open() -> Result<CommandOutput, Error> {
    let session_id = generate_id()?;
    let now = now_iso()?;

    db_exec(
        "INSERT INTO sessions (id, started_at, status) VALUES (?, ?, 'open')",
        vec![json_str(&session_id), json_str(&now)],
    )?;

    log_info(&format!("opened session {session_id}"))?;

    Ok(CommandOutput::ok_with_data(
        format!("Session {session_id} opened."),
        serde_json::json!({ "session_id": session_id }),
    ))
}

fn handle_log(args: &[String]) -> Result<CommandOutput, Error> {
    if args.is_empty() {
        return Ok(CommandOutput::error("usage: bea journal log <message>"));
    }
    let message = args.join(" ");
    let session_id = get_open_session()?;
    let entry_id = generate_id()?;
    let now = now_iso()?;

    db_exec(
        "INSERT INTO entries (id, session_id, created_at, message, tag) VALUES (?, ?, ?, ?, NULL)",
        vec![
            json_str(&entry_id),
            json_str(&session_id),
            json_str(&now),
            json_str(&message),
        ],
    )?;

    Ok(CommandOutput::ok(format!("Logged: {message}")))
}

fn handle_note(args: &[String]) -> Result<CommandOutput, Error> {
    // Parse --tag <tag> <message...>
    let (tag, message) = parse_note_args(args)?;
    let session_id = get_open_session()?;
    let entry_id = generate_id()?;
    let now = now_iso()?;

    db_exec(
        "INSERT INTO entries (id, session_id, created_at, message, tag) VALUES (?, ?, ?, ?, ?)",
        vec![
            json_str(&entry_id),
            json_str(&session_id),
            json_str(&now),
            json_str(&message),
            json_str(&tag),
        ],
    )?;

    Ok(CommandOutput::ok(format!("[{tag}] {message}")))
}

fn handle_summary() -> Result<CommandOutput, Error> {
    let session_id = get_open_session()?;

    let session_resp = db_exec(
        "SELECT started_at, status FROM sessions WHERE id = ?",
        vec![json_str(&session_id)],
    )?;

    let started_at = session_resp
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let entries_resp = db_exec(
        "SELECT created_at, tag, message FROM entries WHERE session_id = ? ORDER BY created_at",
        vec![json_str(&session_id)],
    )?;

    let mut summary = format!("Session: {session_id}\nStarted: {started_at}\n\n");

    if entries_resp.rows.is_empty() {
        summary.push_str("(no entries yet)\n");
    } else {
        for row in &entries_resp.rows {
            let time = row.first().and_then(|v| v.as_str()).unwrap_or("?");
            let tag = row.get(1).and_then(|v| v.as_str());
            let msg = row.get(2).and_then(|v| v.as_str()).unwrap_or("");

            if let Some(t) = tag {
                summary.push_str(&format!("  [{t}] {time}: {msg}\n"));
            } else {
                summary.push_str(&format!("  {time}: {msg}\n"));
            }
        }
    }

    Ok(CommandOutput::ok_with_data(
        summary,
        serde_json::json!({
            "session_id": session_id,
            "entry_count": entries_resp.rows.len(),
        }),
    ))
}

fn handle_close() -> Result<CommandOutput, Error> {
    let session_id = get_open_session()?;
    let now = now_iso()?;

    db_exec(
        "UPDATE sessions SET status = 'closed', closed_at = ? WHERE id = ?",
        vec![json_str(&now), json_str(&session_id)],
    )?;

    log_info(&format!("closed session {session_id}"))?;

    Ok(CommandOutput::ok(format!("Session {session_id} closed.")))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the most recent open session, or return an error.
fn get_open_session() -> Result<String, Error> {
    let resp = db_exec(
        "SELECT id FROM sessions WHERE status = 'open' ORDER BY started_at DESC LIMIT 1",
        vec![],
    )?;

    resp.rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::msg("no open session (run 'bea journal open' first)"))
}

/// Execute SQL via the host_db_exec host function.
fn db_exec(
    sql: &str,
    params: Vec<serde_json::Value>,
) -> Result<DbExecResponse, Error> {
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

fn parse_note_args(args: &[String]) -> Result<(String, String), Error> {
    if args.len() < 3 {
        return Err(Error::msg("usage: bea journal note --tag <tag> <message>"));
    }
    if args[0] != "--tag" {
        return Err(Error::msg(format!(
            "expected --tag as first argument, got '{}'",
            args[0]
        )));
    }
    let tag = args[1].clone();
    let message = args[2..].join(" ");
    Ok((tag, message))
}

/// Generate a unique ID from timestamp + in-process counter.
fn generate_id() -> Result<String, Error> {
    let ts = now_iso()?;
    let counter = var::get("_id_counter")
        .unwrap_or_default()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let next = counter + 1;
    var::set("_id_counter", &next.to_string()).ok();
    // Hash timestamp + counter for a short unique ID.
    let mut h: u64 = 0xcbf29ce484222325;
    for byte in ts.as_bytes().iter().chain(&next.to_le_bytes()) {
        h ^= *byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    Ok(format!("j-{h:016x}"))
}

/// Get current time from host.
fn now_iso() -> Result<String, Error> {
    let ts = unsafe { host_time()? };
    Ok(ts)
}
