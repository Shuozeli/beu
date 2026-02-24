use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use extism::{Function, UserData, PTR};

use crate::db::{PluginDb, PluginLog};

/// Shared context passed to host functions as UserData.
/// Each plugin gets its own HostContext with the correct plugin_name,
/// ensuring DB calls route to the right per-plugin database.
#[derive(Clone)]
pub struct HostContext {
    /// Absolute path to the project root (for fs scoping).
    pub project_root: PathBuf,
    /// Name of the plugin currently executing.
    pub plugin_name: String,
    /// Shared PluginDb instance.
    pub db: Arc<Mutex<PluginDb>>,
    /// Shared PluginLog for persistent log storage (optional).
    pub plugin_log: Option<Arc<Mutex<PluginLog>>>,
}

/// Build all host functions for a given plugin context.
/// Returns a Vec<Function> to be passed to PluginBuilder::with_functions.
pub fn build_host_functions(ctx: HostContext) -> Vec<Function> {
    vec![
        build_host_log(ctx.clone()),
        build_host_fs_read(ctx.clone()),
        build_host_fs_write(ctx.clone()),
        build_host_db_exec(ctx.clone()),
        build_host_time(ctx.clone()),
        build_host_env(ctx),
    ]
}

fn build_host_log(ctx: HostContext) -> Function {
    Function::new(
        "host_log",
        [PTR, PTR],
        [PTR],
        UserData::new(ctx),
        |plugin, inputs, outputs, user_data| {
            let level: String = plugin.memory_get_val(&inputs[0])?;
            let message: String = plugin.memory_get_val(&inputs[1])?;
            let ctx = user_data.get()?;
            let ctx = ctx.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
            eprintln!("[{level}] [{}] {message}", ctx.plugin_name);

            // Persist to log store if available (non-fatal).
            if let Some(ref log) = ctx.plugin_log {
                if let Ok(mut log) = log.lock() {
                    let _ = log.write(&ctx.plugin_name, &level, &message);
                }
            }

            let handle = plugin.memory_new("ok")?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}

fn build_host_fs_read(ctx: HostContext) -> Function {
    Function::new(
        "host_fs_read",
        [PTR],
        [PTR],
        UserData::new(ctx),
        |plugin, inputs, outputs, user_data| {
            let path: String = plugin.memory_get_val(&inputs[0])?;
            let ctx = user_data.get()?;
            let ctx = ctx.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
            let resolved = resolve_sandboxed_path(&ctx.project_root, &path)?;
            let content = std::fs::read_to_string(&resolved)
                .map_err(|e| anyhow::anyhow!("fs_read '{}': {e}", path))?;
            let handle = plugin.memory_new(&content)?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}

fn build_host_fs_write(ctx: HostContext) -> Function {
    Function::new(
        "host_fs_write",
        [PTR, PTR],
        [PTR],
        UserData::new(ctx),
        |plugin, inputs, outputs, user_data| {
            let path: String = plugin.memory_get_val(&inputs[0])?;
            let content: String = plugin.memory_get_val(&inputs[1])?;
            let ctx = user_data.get()?;
            let ctx = ctx.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
            let resolved = resolve_sandboxed_path(&ctx.project_root, &path)?;
            if let Some(parent) = resolved.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("fs_write mkdir '{}': {e}", path))?;
            }
            std::fs::write(&resolved, &content)
                .map_err(|e| anyhow::anyhow!("fs_write '{}': {e}", path))?;
            let handle = plugin.memory_new("ok")?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}

fn build_host_db_exec(ctx: HostContext) -> Function {
    Function::new(
        "host_db_exec",
        [PTR],
        [PTR],
        UserData::new(ctx),
        |plugin, inputs, outputs, user_data| {
            let request_json: String = plugin.memory_get_val(&inputs[0])?;
            let request: bea_sdk::DbExecRequest = match serde_json::from_str(&request_json) {
                Ok(r) => r,
                Err(e) => {
                    let err_resp = bea_sdk::DbExecResponse::err(format!("invalid request JSON: {e}"));
                    let json = serde_json::to_string(&err_resp)
                        .map_err(|e| anyhow::anyhow!("host_db_exec: serialize: {e}"))?;
                    let handle = plugin.memory_new(&json)?;
                    outputs[0] = plugin.memory_to_val(handle);
                    return Ok(());
                }
            };

            let ctx = user_data.get()?;
            let ctx = ctx.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
            let mut db = ctx.db.lock().map_err(|e| anyhow::anyhow!("db lock: {e}"))?;

            // SQL errors are returned as error responses, not host function failures.
            // This lets plugins handle them gracefully instead of trapping.
            let response = match db.exec(&ctx.plugin_name, &request) {
                Ok(r) => r,
                Err(e) => bea_sdk::DbExecResponse::err(e.to_string()),
            };

            let json = serde_json::to_string(&response)
                .map_err(|e| anyhow::anyhow!("host_db_exec: serialize: {e}"))?;
            let handle = plugin.memory_new(&json)?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}

fn build_host_time(ctx: HostContext) -> Function {
    Function::new(
        "host_time",
        [],
        [PTR],
        UserData::new(ctx),
        |plugin, _inputs, outputs, _user_data| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| anyhow::anyhow!("time: {e}"))?;
            let secs = now.as_secs();
            let nanos = now.subsec_nanos();
            // Return ISO 8601 UTC string.
            let ts = format_timestamp(secs, nanos);
            let handle = plugin.memory_new(&ts)?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}

fn build_host_env(ctx: HostContext) -> Function {
    Function::new(
        "host_env",
        [PTR],
        [PTR],
        UserData::new(ctx),
        |plugin, inputs, outputs, user_data| {
            let key: String = plugin.memory_get_val(&inputs[0])?;
            let ctx = user_data.get()?;
            let ctx = ctx.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;

            let value = match key.as_str() {
                "project_root" => ctx.project_root.to_string_lossy().to_string(),
                "plugin_name" => ctx.plugin_name.clone(),
                "bea_version" => env!("CARGO_PKG_VERSION").to_string(),
                _ => String::new(),
            };

            let handle = plugin.memory_new(&value)?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )
}

fn format_timestamp(secs: u64, nanos: u32) -> String {
    // Simple UTC timestamp without external deps.
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    let millis = nanos / 1_000_000;

    // Days since epoch to Y-M-D (simplified Gregorian).
    let (year, month, day) = days_to_ymd(days);

    format!(
        "{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z"
    )
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}


// ---------------------------------------------------------------------------
// Path sandboxing
// ---------------------------------------------------------------------------

/// Resolve a path relative to the project root, rejecting traversal attacks.
fn resolve_sandboxed_path(root: &Path, user_path: &str) -> Result<PathBuf, anyhow::Error> {
    let requested = Path::new(user_path);

    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };

    // Canonicalize to resolve symlinks and `..` components.
    let resolved = if candidate.exists() {
        candidate.canonicalize()?
    } else {
        // For new files, canonicalize the parent.
        let parent = candidate
            .parent()
            .ok_or_else(|| anyhow::anyhow!("path resolve: no parent"))?;
        if !parent.exists() {
            anyhow::bail!("path '{}' is outside the project sandbox", user_path);
        }
        let parent_resolved = parent.canonicalize()?;
        let file_name = candidate
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("path resolve: no filename"))?;
        parent_resolved.join(file_name)
    };

    let root_canonical = root.canonicalize()?;
    if !resolved.starts_with(&root_canonical) {
        anyhow::bail!("path '{}' is outside the project sandbox", user_path);
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sandboxed_path_allows_relative() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        let result = resolve_sandboxed_path(tmp.path(), "file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn sandboxed_path_rejects_traversal() {
        let tmp = TempDir::new().unwrap();
        let result = resolve_sandboxed_path(tmp.path(), "../../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn sandboxed_path_rejects_absolute_outside() {
        let tmp = TempDir::new().unwrap();
        let result = resolve_sandboxed_path(tmp.path(), "/etc/passwd");
        assert!(result.is_err());
    }
}
