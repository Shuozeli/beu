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
        name: "agile".into(),
        version: "0.1.0".into(),
        description: "Task and issue engine. Create, track, and manage work items.".into(),
        commands: vec![
            CommandDef {
                name: "add".into(),
                description: "Create a new task.".into(),
                args: vec![
                    ArgDef {
                        name: "title".into(),
                        description: "Task title.".into(),
                        required: true,
                    },
                    ArgDef {
                        name: "--priority".into(),
                        description: "Priority: low, medium, high, critical (default: medium)."
                            .into(),
                        required: false,
                    },
                    ArgDef {
                        name: "--tag".into(),
                        description: "Tag for categorization (e.g. bug, feature, chore).".into(),
                        required: false,
                    },
                ],
            },
            CommandDef {
                name: "list".into(),
                description: "List tasks with optional filters.".into(),
                args: vec![
                    ArgDef {
                        name: "--status".into(),
                        description: "Filter by status: open, in-progress, done, blocked.".into(),
                        required: false,
                    },
                    ArgDef {
                        name: "--tag".into(),
                        description: "Filter by tag.".into(),
                        required: false,
                    },
                ],
            },
            CommandDef {
                name: "update".into(),
                description: "Update a task's status or priority.".into(),
                args: vec![
                    ArgDef {
                        name: "id".into(),
                        description: "Task ID.".into(),
                        required: true,
                    },
                    ArgDef {
                        name: "--status".into(),
                        description: "New status: open, in-progress, done, blocked.".into(),
                        required: false,
                    },
                    ArgDef {
                        name: "--priority".into(),
                        description: "New priority: low, medium, high, critical.".into(),
                        required: false,
                    },
                ],
            },
            CommandDef {
                name: "done".into(),
                description: "Mark a task as done.".into(),
                args: vec![ArgDef {
                    name: "id".into(),
                    description: "Task ID.".into(),
                    required: true,
                }],
            },
            CommandDef {
                name: "show".into(),
                description: "Show details for a specific task.".into(),
                args: vec![ArgDef {
                    name: "id".into(),
                    description: "Task ID.".into(),
                    required: true,
                }],
            },
            CommandDef {
                name: "sprint".into(),
                description: "Show summary of current open and in-progress tasks.".into(),
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
        "add" => handle_add(&cmd.args)?,
        "list" => handle_list(&cmd.args)?,
        "update" => handle_update(&cmd.args)?,
        "done" => handle_done(&cmd.args)?,
        "show" => handle_show(&cmd.args)?,
        "sprint" => handle_sprint()?,
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
        "CREATE TABLE IF NOT EXISTS tasks (\
            id INTEGER PRIMARY KEY AUTOINCREMENT, \
            title TEXT NOT NULL, \
            status TEXT NOT NULL DEFAULT 'open', \
            priority TEXT NOT NULL DEFAULT 'medium', \
            tag TEXT, \
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
        return Ok(CommandOutput::error(
            "usage: bea agile add <title> [--priority <p>] [--tag <t>]",
        ));
    }

    let parsed = parse_add_args(args);
    let now = now_iso()?;

    db_exec(
        "INSERT INTO tasks (title, status, priority, tag, created_at, updated_at) VALUES (?, 'open', ?, ?, ?, ?)",
        vec![
            json_str(&parsed.title),
            json_str(&parsed.priority),
            parsed.tag.as_ref().map_or(serde_json::Value::Null, |t| json_str(t)),
            json_str(&now),
            json_str(&now),
        ],
    )?;

    // Get the ID of the inserted row (each db_exec opens a new connection,
    // so last_insert_rowid() won't work -- use MAX(id) instead).
    let id_resp = db_exec("SELECT MAX(id) FROM tasks", vec![])?;
    let id = id_resp
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    log_info(&format!("created task #{id}: {}", parsed.title))?;

    let tag_info = parsed
        .tag
        .as_ref()
        .map(|t| format!(" [{t}]"))
        .unwrap_or_default();

    Ok(CommandOutput::ok_with_data(
        format!("#{id}: {}{tag_info} ({})", parsed.title, parsed.priority),
        serde_json::json!({
            "id": id,
            "title": parsed.title,
            "priority": parsed.priority,
            "tag": parsed.tag,
            "status": "open",
        }),
    ))
}

fn handle_list(args: &[String]) -> Result<CommandOutput, Error> {
    let filters = parse_list_filters(args);

    let (sql, params) = build_list_query(&filters);
    let resp = db_exec(&sql, params)?;

    if resp.rows.is_empty() {
        return Ok(CommandOutput::ok("No tasks found."));
    }

    let mut output = String::new();
    let mut items = Vec::new();

    for row in &resp.rows {
        let id = row.first().and_then(|v| v.as_i64()).unwrap_or(0);
        let title = row.get(1).and_then(|v| v.as_str()).unwrap_or("?");
        let status = row.get(2).and_then(|v| v.as_str()).unwrap_or("?");
        let priority = row.get(3).and_then(|v| v.as_str()).unwrap_or("?");
        let tag = row.get(4).and_then(|v| v.as_str());

        let tag_str = tag.map(|t| format!(" [{t}]")).unwrap_or_default();
        let pri_marker = match priority {
            "critical" => "!!!",
            "high" => "!! ",
            "medium" => "!  ",
            _ => "   ",
        };

        output.push_str(&format!(
            "  {pri_marker} #{id} [{status}]{tag_str} {title}\n"
        ));
        items.push(serde_json::json!({
            "id": id,
            "title": title,
            "status": status,
            "priority": priority,
            "tag": tag,
        }));
    }

    Ok(CommandOutput::ok_with_data(
        output,
        serde_json::json!({
            "count": items.len(),
            "tasks": items,
        }),
    ))
}

fn handle_update(args: &[String]) -> Result<CommandOutput, Error> {
    if args.is_empty() {
        return Ok(CommandOutput::error(
            "usage: bea agile update <id> [--status <s>] [--priority <p>]",
        ));
    }

    let id_str = &args[0];
    let id: i64 = id_str
        .trim_start_matches('#')
        .parse()
        .map_err(|_| Error::msg(format!("invalid task ID: {id_str}")))?;

    // Check exists.
    let existing = db_exec(
        "SELECT title, status, priority FROM tasks WHERE id = ?",
        vec![serde_json::json!(id)],
    )?;
    if existing.rows.is_empty() {
        return Ok(CommandOutput::error(format!("task #{id} not found")));
    }

    let old_status = existing.rows[0]
        .get(1)
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let old_priority = existing.rows[0]
        .get(2)
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    let new_status = parse_flag(args, "--status");
    let new_priority = parse_flag(args, "--priority");

    if new_status.is_none() && new_priority.is_none() {
        return Ok(CommandOutput::error(
            "nothing to update. Use --status or --priority.",
        ));
    }

    // Validate values.
    if let Some(s) = &new_status {
        let valid = ["open", "in-progress", "done", "blocked"];
        if !valid.contains(&s.as_str()) {
            return Ok(CommandOutput::error(format!(
                "invalid status '{s}' (valid: {})",
                valid.join(", ")
            )));
        }
    }
    if let Some(p) = &new_priority {
        let valid = ["low", "medium", "high", "critical"];
        if !valid.contains(&p.as_str()) {
            return Ok(CommandOutput::error(format!(
                "invalid priority '{p}' (valid: {})",
                valid.join(", ")
            )));
        }
    }

    let now = now_iso()?;
    let status = new_status.as_deref().unwrap_or(old_status);
    let priority = new_priority.as_deref().unwrap_or(old_priority);

    db_exec(
        "UPDATE tasks SET status = ?, priority = ?, updated_at = ? WHERE id = ?",
        vec![
            json_str(status),
            json_str(priority),
            json_str(&now),
            serde_json::json!(id),
        ],
    )?;

    let mut changes = Vec::new();
    if new_status.is_some() {
        changes.push(format!("status: {old_status} -> {status}"));
    }
    if new_priority.is_some() {
        changes.push(format!("priority: {old_priority} -> {priority}"));
    }

    Ok(CommandOutput::ok(format!(
        "#{id}: {}",
        changes.join(", ")
    )))
}

fn handle_done(args: &[String]) -> Result<CommandOutput, Error> {
    if args.is_empty() {
        return Ok(CommandOutput::error("usage: bea agile done <id>"));
    }

    let id_str = &args[0];
    let id: i64 = id_str
        .trim_start_matches('#')
        .parse()
        .map_err(|_| Error::msg(format!("invalid task ID: {id_str}")))?;

    let existing = db_exec(
        "SELECT title FROM tasks WHERE id = ?",
        vec![serde_json::json!(id)],
    )?;
    if existing.rows.is_empty() {
        return Ok(CommandOutput::error(format!("task #{id} not found")));
    }

    let title = existing.rows[0]
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    let now = now_iso()?;
    db_exec(
        "UPDATE tasks SET status = 'done', updated_at = ? WHERE id = ?",
        vec![json_str(&now), serde_json::json!(id)],
    )?;

    log_info(&format!("completed task #{id}: {title}"))?;

    Ok(CommandOutput::ok(format!("#{id} done: {title}")))
}

fn handle_show(args: &[String]) -> Result<CommandOutput, Error> {
    if args.is_empty() {
        return Ok(CommandOutput::error("usage: bea agile show <id>"));
    }

    let id_str = &args[0];
    let id: i64 = id_str
        .trim_start_matches('#')
        .parse()
        .map_err(|_| Error::msg(format!("invalid task ID: {id_str}")))?;

    let resp = db_exec(
        "SELECT id, title, status, priority, tag, created_at, updated_at FROM tasks WHERE id = ?",
        vec![serde_json::json!(id)],
    )?;

    if resp.rows.is_empty() {
        return Ok(CommandOutput::error(format!("task #{id} not found")));
    }

    let row = &resp.rows[0];
    let title = row.get(1).and_then(|v| v.as_str()).unwrap_or("?");
    let status = row.get(2).and_then(|v| v.as_str()).unwrap_or("?");
    let priority = row.get(3).and_then(|v| v.as_str()).unwrap_or("?");
    let tag = row.get(4).and_then(|v| v.as_str());
    let created = row.get(5).and_then(|v| v.as_str()).unwrap_or("?");
    let updated = row.get(6).and_then(|v| v.as_str()).unwrap_or("?");

    let tag_line = tag
        .map(|t| format!("\nTag: {t}"))
        .unwrap_or_default();

    let msg = format!(
        "Task #{id}: {title}\nStatus: {status}\nPriority: {priority}{tag_line}\nCreated: {created}\nUpdated: {updated}"
    );

    Ok(CommandOutput::ok_with_data(
        msg,
        serde_json::json!({
            "id": id,
            "title": title,
            "status": status,
            "priority": priority,
            "tag": tag,
            "created_at": created,
            "updated_at": updated,
        }),
    ))
}

fn handle_sprint() -> Result<CommandOutput, Error> {
    let resp = db_exec(
        "SELECT id, title, status, priority, tag FROM tasks WHERE status IN ('open', 'in-progress', 'blocked') ORDER BY \
         CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END, id",
        vec![],
    )?;

    if resp.rows.is_empty() {
        return Ok(CommandOutput::ok(
            "Sprint is clear -- no open or in-progress tasks.",
        ));
    }

    let mut in_progress = Vec::new();
    let mut blocked = Vec::new();
    let mut open = Vec::new();

    for row in &resp.rows {
        let id = row.first().and_then(|v| v.as_i64()).unwrap_or(0);
        let title = row.get(1).and_then(|v| v.as_str()).unwrap_or("?");
        let status = row.get(2).and_then(|v| v.as_str()).unwrap_or("?");
        let priority = row.get(3).and_then(|v| v.as_str()).unwrap_or("?");
        let tag = row.get(4).and_then(|v| v.as_str());

        let tag_str = tag.map(|t| format!(" [{t}]")).unwrap_or_default();
        let line = format!("  #{id} ({priority}){tag_str} {title}");

        match status {
            "in-progress" => in_progress.push(line),
            "blocked" => blocked.push(line),
            _ => open.push(line),
        }
    }

    let mut output = String::new();

    if !in_progress.is_empty() {
        output.push_str("In Progress:\n");
        for l in &in_progress {
            output.push_str(l);
            output.push('\n');
        }
        output.push('\n');
    }

    if !blocked.is_empty() {
        output.push_str("Blocked:\n");
        for l in &blocked {
            output.push_str(l);
            output.push('\n');
        }
        output.push('\n');
    }

    if !open.is_empty() {
        output.push_str("Open:\n");
        for l in &open {
            output.push_str(l);
            output.push('\n');
        }
    }

    let total = in_progress.len() + blocked.len() + open.len();

    Ok(CommandOutput::ok_with_data(
        output,
        serde_json::json!({
            "in_progress": in_progress.len(),
            "blocked": blocked.len(),
            "open": open.len(),
            "total": total,
        }),
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct AddArgs {
    title: String,
    priority: String,
    tag: Option<String>,
}

fn parse_add_args(args: &[String]) -> AddArgs {
    let mut title_parts = Vec::new();
    let mut priority = "medium".to_string();
    let mut tag = None;
    let mut i = 0;

    while i < args.len() {
        if args[i] == "--priority" && i + 1 < args.len() {
            priority = args[i + 1].clone();
            i += 2;
        } else if args[i] == "--tag" && i + 1 < args.len() {
            tag = Some(args[i + 1].clone());
            i += 2;
        } else {
            title_parts.push(args[i].as_str());
            i += 1;
        }
    }

    AddArgs {
        title: title_parts.join(" "),
        priority,
        tag,
    }
}

struct ListFilters {
    status: Option<String>,
    tag: Option<String>,
}

fn parse_list_filters(args: &[String]) -> ListFilters {
    ListFilters {
        status: parse_flag(args, "--status"),
        tag: parse_flag(args, "--tag"),
    }
}

fn parse_flag(args: &[String], flag: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        i += 1;
    }
    None
}

fn build_list_query(filters: &ListFilters) -> (String, Vec<serde_json::Value>) {
    let mut conditions = Vec::new();
    let mut params = Vec::new();

    if let Some(status) = &filters.status {
        conditions.push("status = ?");
        params.push(json_str(status));
    }
    if let Some(tag) = &filters.tag {
        conditions.push("tag = ?");
        params.push(json_str(tag));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, title, status, priority, tag FROM tasks{where_clause} ORDER BY \
         CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END, id"
    );
    (sql, params)
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
