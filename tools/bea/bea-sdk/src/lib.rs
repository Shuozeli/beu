use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Plugin Metadata (returned by the `metadata()` Wasm export)
// ---------------------------------------------------------------------------

/// Top-level metadata a plugin returns so the host can discover its commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Short, lowercase name used as the CLI sub-command (e.g. "journal").
    pub name: String,
    /// Semantic version string (e.g. "0.1.0").
    pub version: String,
    /// One-line human-readable description.
    pub description: String,
    /// Commands the plugin exposes.
    pub commands: Vec<CommandDef>,
}

/// Definition of a single command within a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    /// Command name (e.g. "open", "log").
    pub name: String,
    /// Human-readable description shown in help and skill.md.
    pub description: String,
    /// Positional and flag arguments the command accepts.
    pub args: Vec<ArgDef>,
}

/// Definition of a single argument for a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgDef {
    /// Argument name (e.g. "message", "--tag").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Whether the argument must be provided.
    pub required: bool,
}

// ---------------------------------------------------------------------------
// Command Dispatch (host -> plugin via `run_command()`)
// ---------------------------------------------------------------------------

/// JSON payload the host sends to `run_command()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInput {
    /// The command name to execute (e.g. "open").
    pub command: String,
    /// Arguments passed after the command.
    pub args: Vec<String>,
}

/// JSON payload `run_command()` returns to the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    /// Whether the command succeeded or failed.
    pub status: CommandStatus,
    /// Human-readable message (shown to user / agent).
    pub message: String,
    /// Optional structured data for programmatic consumption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Command execution result status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandStatus {
    Ok,
    Error,
}

impl CommandOutput {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            status: CommandStatus::Ok,
            message: message.into(),
            data: None,
        }
    }

    pub fn ok_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            status: CommandStatus::Ok,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: CommandStatus::Error,
            message: message.into(),
            data: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Host DB Function ABI (plugin -> host via `host_db_exec`)
// ---------------------------------------------------------------------------

/// Request payload for the `host_db_exec` host function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbExecRequest {
    /// SQL statement to execute.
    pub sql: String,
    /// Bind parameters (positional, matched to `?` placeholders).
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
}

/// Response payload from the `host_db_exec` host function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbExecResponse {
    /// Column names for SELECT results.
    #[serde(default)]
    pub columns: Vec<String>,
    /// Rows of values (each row is a JSON array matching `columns`).
    #[serde(default)]
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Number of rows affected (for INSERT/UPDATE/DELETE).
    pub rows_affected: u64,
    /// Error message if the SQL execution failed.
    /// When set, columns/rows/rows_affected should be ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl DbExecResponse {
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: 0,
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: 0,
            error: Some(message.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_output_ok_serialization() {
        let out = CommandOutput::ok("done");
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""status":"ok"#));
        assert!(json.contains(r#""message":"done"#));
        // data should be omitted when None
        assert!(!json.contains("data"));
    }

    #[test]
    fn command_output_error_serialization() {
        let out = CommandOutput::error("bad input");
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""status":"error"#));
    }

    #[test]
    fn db_exec_request_roundtrip() {
        let req = DbExecRequest {
            sql: "SELECT * FROM foo WHERE id = ?".into(),
            params: vec![serde_json::json!("abc")],
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: DbExecRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.sql, req.sql);
        assert_eq!(decoded.params.len(), 1);
    }

    #[test]
    fn plugin_metadata_roundtrip() {
        let meta = PluginMetadata {
            name: "journal".into(),
            version: "0.1.0".into(),
            description: "Agent interaction ledger".into(),
            commands: vec![CommandDef {
                name: "open".into(),
                description: "Start a new session".into(),
                args: vec![],
            }],
        };
        let json = serde_json::to_string(&meta).unwrap();
        let decoded: PluginMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.name, "journal");
        assert_eq!(decoded.commands.len(), 1);
    }
}
