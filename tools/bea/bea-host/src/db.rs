use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use bea_sdk::{DbExecRequest, DbExecResponse};

// ---------------------------------------------------------------------------
// Plugin Log (persistent host_log storage)
// ---------------------------------------------------------------------------

/// Persists host_log calls from plugins in a shared SQLite database.
pub struct PluginLog {
    conn: Connection,
}

/// A single log entry.
#[derive(Debug)]
pub struct LogEntry {
    pub id: i64,
    pub timestamp: String,
    pub plugin: String,
    pub level: String,
    pub message: String,
}

impl PluginLog {
    /// Open (or create) the plugin log database at `.bea/data/_logs.db`.
    pub fn open(bea_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = bea_dir.join("data");
        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("_logs.db");
        let mut conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                plugin TEXT NOT NULL,
                level TEXT NOT NULL,
                message TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_logs_plugin ON logs(plugin);
            CREATE INDEX IF NOT EXISTS idx_logs_level ON logs(level);",
        )?;
        tx.commit()?;

        Ok(Self { conn })
    }

    /// Record a log entry.
    pub fn write(
        &mut self,
        plugin: &str,
        level: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO logs (timestamp, plugin, level, message) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![timestamp, plugin, level, message],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Query recent log entries, most recent first.
    pub fn recent(
        &mut self,
        limit: usize,
        plugin_filter: Option<&str>,
        level_filter: Option<&str>,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let entries = match (plugin_filter, level_filter) {
            (Some(p), Some(l)) => {
                let mut stmt = tx.prepare(
                    "SELECT id, timestamp, plugin, level, message FROM logs
                     WHERE plugin = ?1 AND level = ?2 ORDER BY id DESC LIMIT ?3",
                )?;
                let result = stmt
                    .query_map(rusqlite::params![p, l, limit as i64], row_to_log_entry)?
                    .collect::<Result<Vec<_>, _>>()?;
                result
            }
            (Some(p), None) => {
                let mut stmt = tx.prepare(
                    "SELECT id, timestamp, plugin, level, message FROM logs
                     WHERE plugin = ?1 ORDER BY id DESC LIMIT ?2",
                )?;
                let result = stmt
                    .query_map(rusqlite::params![p, limit as i64], row_to_log_entry)?
                    .collect::<Result<Vec<_>, _>>()?;
                result
            }
            (None, Some(l)) => {
                let mut stmt = tx.prepare(
                    "SELECT id, timestamp, plugin, level, message FROM logs
                     WHERE level = ?1 ORDER BY id DESC LIMIT ?2",
                )?;
                let result = stmt
                    .query_map(rusqlite::params![l, limit as i64], row_to_log_entry)?
                    .collect::<Result<Vec<_>, _>>()?;
                result
            }
            (None, None) => {
                let mut stmt = tx.prepare(
                    "SELECT id, timestamp, plugin, level, message FROM logs
                     ORDER BY id DESC LIMIT ?1",
                )?;
                let result = stmt
                    .query_map(rusqlite::params![limit as i64], row_to_log_entry)?
                    .collect::<Result<Vec<_>, _>>()?;
                result
            }
        };

        tx.commit()?;
        Ok(entries)
    }
}

fn row_to_log_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<LogEntry> {
    Ok(LogEntry {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        plugin: row.get(2)?,
        level: row.get(3)?,
        message: row.get(4)?,
    })
}

// ---------------------------------------------------------------------------
// Event Log
// ---------------------------------------------------------------------------

/// Records all state-changing actions in a shared event log database.
/// Used for debugging, auditing, and observability.
pub struct EventLog {
    conn: Connection,
}

/// A single logged event.
#[derive(Debug)]
pub struct Event {
    pub id: i64,
    pub timestamp: String,
    pub plugin: String,
    pub command: String,
    pub args: String,
    pub status: String,
    pub duration_ms: i64,
}

impl EventLog {
    /// Open (or create) the event log database at `.bea/data/_events.db`.
    pub fn open(bea_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = bea_dir.join("data");
        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("_events.db");
        let mut conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                plugin TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                duration_ms INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_plugin ON events(plugin);",
        )?;
        tx.commit()?;

        Ok(Self { conn })
    }

    /// Record an event.
    pub fn log(
        &mut self,
        plugin: &str,
        command: &str,
        args: &[String],
        status: &str,
        duration_ms: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let args_str = args.join(" ");
        let timestamp = utc_now();

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO events (timestamp, plugin, command, args, status, duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![timestamp, plugin, command, args_str, status, duration_ms],
        )?;
        tx.commit()?;

        Ok(())
    }

    /// Query recent events, most recent first.
    pub fn recent(
        &mut self,
        limit: usize,
        plugin_filter: Option<&str>,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let events = if let Some(plugin) = plugin_filter {
            let mut stmt = tx.prepare(
                "SELECT id, timestamp, plugin, command, args, status, duration_ms
                 FROM events WHERE plugin = ?1 ORDER BY id DESC LIMIT ?2",
            )?;
            let result = stmt
                .query_map(rusqlite::params![plugin, limit as i64], row_to_event)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        } else {
            let mut stmt = tx.prepare(
                "SELECT id, timestamp, plugin, command, args, status, duration_ms
                 FROM events ORDER BY id DESC LIMIT ?1",
            )?;
            let result = stmt
                .query_map(rusqlite::params![limit as i64], row_to_event)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(events)
    }

    /// Count total events in the log.
    pub fn count(&mut self) -> Result<i64, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?;
        tx.commit()?;
        Ok(count)
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        plugin: row.get(2)?,
        command: row.get(3)?,
        args: row.get(4)?,
        status: row.get(5)?,
        duration_ms: row.get(6)?,
    })
}

fn utc_now() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z")
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
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

/// Manages per-plugin SQLite databases under `.bea/data/<plugin_name>.db`.
/// Keeps connections open across calls for the same plugin (connection pooling).
pub struct PluginDb {
    db_dir: PathBuf,
    connections: HashMap<String, Connection>,
}

impl PluginDb {
    pub fn new(bea_dir: &Path) -> Self {
        Self {
            db_dir: bea_dir.join("data"),
            connections: HashMap::new(),
        }
    }

    /// Ensure the data directory exists.
    pub fn ensure_dir(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.db_dir)?;
        Ok(())
    }

    /// Get or create a connection for the named plugin.
    fn connection(
        &mut self,
        plugin_name: &str,
    ) -> Result<&mut Connection, Box<dyn std::error::Error>> {
        if !self.connections.contains_key(plugin_name) {
            let db_path = self.db_dir.join(format!("{plugin_name}.db"));
            let conn = Connection::open(&db_path)?;
            conn.execute_batch("PRAGMA journal_mode=WAL;")?;
            self.connections.insert(plugin_name.to_string(), conn);
        }
        self.connections
            .get_mut(plugin_name)
            .ok_or_else(|| format!("connection not found for plugin '{plugin_name}'").into())
    }

    /// Execute a SQL statement against the named plugin's database.
    /// All operations are wrapped in a transaction.
    /// Connections are cached per plugin name for the lifetime of PluginDb.
    pub fn exec(
        &mut self,
        plugin_name: &str,
        request: &DbExecRequest,
    ) -> Result<DbExecResponse, Box<dyn std::error::Error>> {
        let conn = self.connection(plugin_name)?;
        let tx = conn.transaction()?;
        let result = exec_in_transaction(&tx, request)?;
        tx.commit()?;

        Ok(result)
    }

    /// Export all data from a plugin's database as JSON.
    /// Returns a map of table_name -> array of row objects.
    pub fn export_plugin(
        &mut self,
        plugin_name: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let db_path = self.db_dir.join(format!("{plugin_name}.db"));
        if !db_path.exists() {
            return Err(format!("no database found for plugin '{plugin_name}'").into());
        }

        let conn = self.connection(plugin_name)?;
        let tx = conn.transaction()?;

        // Get all user tables.
        let mut stmt =
            tx.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        let mut result = serde_json::Map::new();

        for table_name in &tables {
            let mut stmt = tx.prepare(&format!("SELECT * FROM [{table_name}]"))?;
            let column_count = stmt.column_count();
            let columns: Vec<String> = (0..column_count)
                .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                .collect();

            let rows: Vec<serde_json::Value> = stmt
                .query_map([], |row| {
                    let mut obj = serde_json::Map::new();
                    for (i, col) in columns.iter().enumerate() {
                        obj.insert(col.clone(), sqlite_value_to_json(row, i));
                    }
                    Ok(serde_json::Value::Object(obj))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            result.insert(table_name.clone(), serde_json::Value::Array(rows));
        }

        tx.commit()?;

        Ok(serde_json::Value::Object(result))
    }

    /// List plugin names that have database files.
    pub fn list_plugin_dbs(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if !self.db_dir.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.db_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "db" {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        // Skip internal databases like _events.db
                        if !stem.starts_with('_') {
                            names.push(stem.to_string());
                        }
                    }
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// Import data from a JSON structure (as produced by export_plugin) into a plugin's database.
    /// Creates tables if they don't exist, inserts all rows.
    /// Returns (tables_imported, rows_imported).
    pub fn import_plugin(
        &mut self,
        plugin_name: &str,
        data: &serde_json::Value,
    ) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        let obj = data
            .as_object()
            .ok_or("import data must be a JSON object with table names as keys")?;

        let conn = self.connection(plugin_name)?;
        let tx = conn.transaction()?;

        let mut tables_imported = 0usize;
        let mut rows_imported = 0usize;

        for (table_name, rows_val) in obj {
            let rows = rows_val
                .as_array()
                .ok_or_else(|| format!("table '{table_name}' value must be an array"))?;

            if rows.is_empty() {
                continue;
            }

            // Get column names from first row.
            let first_row = rows[0]
                .as_object()
                .ok_or_else(|| format!("rows in '{table_name}' must be objects"))?;
            let columns: Vec<&String> = first_row.keys().collect();

            // Create table if it doesn't exist (all TEXT columns).
            let col_defs: Vec<String> = columns.iter().map(|c| format!("[{c}] TEXT")).collect();
            let create_sql =
                format!("CREATE TABLE IF NOT EXISTS [{table_name}] ({})", col_defs.join(", "));
            tx.execute(&create_sql, [])?;

            // Prepare INSERT statement.
            let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{i}")).collect();
            let col_names: Vec<String> = columns.iter().map(|c| format!("[{c}]")).collect();
            let insert_sql = format!(
                "INSERT OR REPLACE INTO [{table_name}] ({}) VALUES ({})",
                col_names.join(", "),
                placeholders.join(", ")
            );
            let mut stmt = tx.prepare(&insert_sql)?;

            for row in rows {
                let row_obj = row
                    .as_object()
                    .ok_or_else(|| format!("rows in '{table_name}' must be objects"))?;

                let params: Vec<Box<dyn rusqlite::types::ToSql>> = columns
                    .iter()
                    .map(|col| {
                        let val = row_obj.get(*col).unwrap_or(&serde_json::Value::Null);
                        json_value_to_sql_param(val)
                    })
                    .collect();

                let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                    params.iter().map(|p| p.as_ref()).collect();
                stmt.execute(param_refs.as_slice())?;
                rows_imported += 1;
            }

            tables_imported += 1;
        }

        tx.commit()?;
        Ok((tables_imported, rows_imported))
    }

    /// Drop all user tables in a plugin's database, resetting it to empty.
    /// Returns the number of tables dropped.
    pub fn reset_plugin(
        &mut self,
        plugin_name: &str,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let db_path = self.db_dir.join(format!("{plugin_name}.db"));
        if !db_path.exists() {
            return Err(format!("no database found for plugin '{plugin_name}'").into());
        }

        let conn = self.connection(plugin_name)?;
        let tx = conn.transaction()?;

        // Get all user tables.
        let mut stmt = tx.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        for table_name in &tables {
            tx.execute_batch(&format!("DROP TABLE IF EXISTS [{table_name}]"))?;
        }

        tx.commit()?;
        Ok(tables.len())
    }

    /// Get the database file size for a plugin (for status reporting).
    pub fn db_size(&self, plugin_name: &str) -> Option<u64> {
        let db_path = self.db_dir.join(format!("{plugin_name}.db"));
        std::fs::metadata(&db_path).ok().map(|m| m.len())
    }
}

fn exec_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    request: &DbExecRequest,
) -> Result<DbExecResponse, Box<dyn std::error::Error>> {
    let sql = request.sql.trim();

    // Determine if this is a query (SELECT) or a mutation.
    let is_query = sql
        .split_whitespace()
        .next()
        .map(|w| w.eq_ignore_ascii_case("SELECT"))
        .unwrap_or(false);

    let params: Vec<Box<dyn rusqlite::types::ToSql>> = request
        .params
        .iter()
        .map(json_value_to_sql_param)
        .collect();

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    if is_query {
        let mut stmt = tx.prepare(sql)?;

        let column_count = stmt.column_count();
        let columns: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
            .collect();

        let rows: Vec<Vec<serde_json::Value>> = stmt
            .query_map(param_refs.as_slice(), |row| {
                let mut values = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    let val = sqlite_value_to_json(row, i);
                    values.push(val);
                }
                Ok(values)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(DbExecResponse {
            columns,
            rows,
            rows_affected: 0,
            error: None,
        })
    } else {
        let affected = tx.execute(sql, param_refs.as_slice())?;
        Ok(DbExecResponse {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: affected as u64,
            error: None,
        })
    }
}

fn json_value_to_sql_param(val: &serde_json::Value) -> Box<dyn rusqlite::types::ToSql> {
    match val {
        serde_json::Value::Null => Box::new(rusqlite::types::Null),
        serde_json::Value::Bool(b) => Box::new(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else if let Some(f) = n.as_f64() {
                Box::new(f)
            } else {
                Box::new(n.to_string())
            }
        }
        serde_json::Value::String(s) => Box::new(s.clone()),
        other => Box::new(other.to_string()),
    }
}

fn sqlite_value_to_json(row: &rusqlite::Row<'_>, idx: usize) -> serde_json::Value {
    // Try types in order: integer, real, text, blob, null.
    if let Ok(v) = row.get::<_, i64>(idx) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(v) = row.get::<_, f64>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.get::<_, String>(idx) {
        return serde_json::Value::String(v);
    }
    if let Ok(v) = row.get::<_, Vec<u8>>(idx) {
        use std::fmt::Write;
        let mut hex = String::with_capacity(v.len() * 2);
        for byte in &v {
            write!(&mut hex, "{byte:02x}").ok();
        }
        return serde_json::Value::String(hex);
    }
    serde_json::Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PluginDb) {
        let tmp = TempDir::new().unwrap();
        let bea_dir = tmp.path().join(".bea");
        std::fs::create_dir_all(bea_dir.join("data")).unwrap();
        let db = PluginDb::new(&bea_dir);
        (tmp, db)
    }

    #[test]
    fn create_table_and_insert() {
        let (_tmp, mut db) = setup();

        // Create table.
        db.exec(
            "test_plugin",
            &DbExecRequest {
                sql: "CREATE TABLE items (id TEXT, name TEXT)".into(),
                params: vec![],
            },
        )
        .unwrap();

        // Insert.
        let res = db
            .exec(
                "test_plugin",
                &DbExecRequest {
                    sql: "INSERT INTO items (id, name) VALUES (?, ?)".into(),
                    params: vec![
                        serde_json::json!("1"),
                        serde_json::json!("hello"),
                    ],
                },
            )
            .unwrap();
        assert_eq!(res.rows_affected, 1);

        // Select.
        let res = db
            .exec(
                "test_plugin",
                &DbExecRequest {
                    sql: "SELECT id, name FROM items".into(),
                    params: vec![],
                },
            )
            .unwrap();
        assert_eq!(res.columns, vec!["id", "name"]);
        assert_eq!(res.rows.len(), 1);
        assert_eq!(res.rows[0][1], serde_json::json!("hello"));
    }

    #[test]
    fn per_plugin_isolation() {
        let (_tmp, mut db) = setup();

        db.exec(
            "alpha",
            &DbExecRequest {
                sql: "CREATE TABLE t (v TEXT)".into(),
                params: vec![],
            },
        )
        .unwrap();

        db.exec(
            "alpha",
            &DbExecRequest {
                sql: "INSERT INTO t (v) VALUES (?)".into(),
                params: vec![serde_json::json!("from_alpha")],
            },
        )
        .unwrap();

        // beta should have its own empty database.
        db.exec(
            "beta",
            &DbExecRequest {
                sql: "CREATE TABLE t (v TEXT)".into(),
                params: vec![],
            },
        )
        .unwrap();

        let res = db
            .exec(
                "beta",
                &DbExecRequest {
                    sql: "SELECT * FROM t".into(),
                    params: vec![],
                },
            )
            .unwrap();
        assert_eq!(res.rows.len(), 0);
    }

    #[test]
    fn connection_reuse_preserves_state() {
        let (_tmp, mut db) = setup();

        // Create table and insert on first call.
        db.exec(
            "reuse_test",
            &DbExecRequest {
                sql: "CREATE TABLE counter (id INTEGER PRIMARY KEY AUTOINCREMENT, val TEXT)"
                    .into(),
                params: vec![],
            },
        )
        .unwrap();

        db.exec(
            "reuse_test",
            &DbExecRequest {
                sql: "INSERT INTO counter (val) VALUES (?)".into(),
                params: vec![serde_json::json!("first")],
            },
        )
        .unwrap();

        // Second insert on the same connection should get autoincrement=2.
        db.exec(
            "reuse_test",
            &DbExecRequest {
                sql: "INSERT INTO counter (val) VALUES (?)".into(),
                params: vec![serde_json::json!("second")],
            },
        )
        .unwrap();

        let res = db
            .exec(
                "reuse_test",
                &DbExecRequest {
                    sql: "SELECT id, val FROM counter ORDER BY id".into(),
                    params: vec![],
                },
            )
            .unwrap();
        assert_eq!(res.rows.len(), 2);
        assert_eq!(res.rows[0][0], serde_json::json!(1));
        assert_eq!(res.rows[1][0], serde_json::json!(2));
    }

    #[test]
    fn db_size_returns_some_for_existing_db() {
        let (_tmp, mut db) = setup();
        db.exec(
            "size_test",
            &DbExecRequest {
                sql: "CREATE TABLE t (v TEXT)".into(),
                params: vec![],
            },
        )
        .unwrap();

        let size = db.db_size("size_test");
        assert!(size.is_some());
        assert!(size.unwrap() > 0);
    }

    #[test]
    fn db_size_returns_none_for_missing_db() {
        let (_tmp, db) = setup();
        assert!(db.db_size("nonexistent").is_none());
    }

    #[test]
    fn event_log_records_and_queries() {
        let tmp = TempDir::new().unwrap();
        let bea_dir = tmp.path().join(".bea");
        std::fs::create_dir_all(bea_dir.join("data")).unwrap();

        let mut log = EventLog::open(&bea_dir).unwrap();
        log.log("journal", "open", &[], "ok", 12).unwrap();
        log.log("journal", "log", &["hello world".into()], "ok", 5)
            .unwrap();
        log.log("agile", "add", &["task one".into()], "ok", 8)
            .unwrap();

        let events = log.recent(10, None).unwrap();
        assert_eq!(events.len(), 3);
        // Most recent first.
        assert_eq!(events[0].plugin, "agile");
        assert_eq!(events[1].command, "log");
        assert_eq!(events[2].command, "open");
    }

    #[test]
    fn event_log_filters_by_plugin() {
        let tmp = TempDir::new().unwrap();
        let bea_dir = tmp.path().join(".bea");
        std::fs::create_dir_all(bea_dir.join("data")).unwrap();

        let mut log = EventLog::open(&bea_dir).unwrap();
        log.log("journal", "open", &[], "ok", 1).unwrap();
        log.log("track", "add", &["doc".into()], "ok", 2).unwrap();
        log.log("journal", "close", &[], "ok", 1).unwrap();

        let events = log.recent(10, Some("journal")).unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| e.plugin == "journal"));
    }

    #[test]
    fn event_log_respects_limit() {
        let tmp = TempDir::new().unwrap();
        let bea_dir = tmp.path().join(".bea");
        std::fs::create_dir_all(bea_dir.join("data")).unwrap();

        let mut log = EventLog::open(&bea_dir).unwrap();
        for i in 0..20 {
            log.log("test", &format!("cmd{i}"), &[], "ok", 1).unwrap();
        }

        let events = log.recent(5, None).unwrap();
        assert_eq!(events.len(), 5);
        // Most recent (cmd19) should be first.
        assert_eq!(events[0].command, "cmd19");
    }

    #[test]
    fn event_log_count() {
        let tmp = TempDir::new().unwrap();
        let bea_dir = tmp.path().join(".bea");
        std::fs::create_dir_all(bea_dir.join("data")).unwrap();

        let mut log = EventLog::open(&bea_dir).unwrap();
        assert_eq!(log.count().unwrap(), 0);

        log.log("journal", "open", &[], "ok", 5).unwrap();
        log.log("journal", "log", &["hi".into()], "ok", 3).unwrap();
        assert_eq!(log.count().unwrap(), 2);
    }

    #[test]
    fn ensure_dir_creates_data_directory() {
        let tmp = TempDir::new().unwrap();
        let bea_dir = tmp.path().join(".bea");
        // Don't pre-create data dir.
        let db = PluginDb::new(&bea_dir);
        assert!(!bea_dir.join("data").exists());

        db.ensure_dir().unwrap();
        assert!(bea_dir.join("data").is_dir());
    }

    #[test]
    fn list_plugin_dbs_returns_db_names() {
        let (_tmp, mut db) = setup();

        // Create databases for two plugins.
        db.exec(
            "alpha",
            &DbExecRequest {
                sql: "CREATE TABLE t (v TEXT)".into(),
                params: vec![],
            },
        )
        .unwrap();
        db.exec(
            "beta",
            &DbExecRequest {
                sql: "CREATE TABLE t (v TEXT)".into(),
                params: vec![],
            },
        )
        .unwrap();

        let names = db.list_plugin_dbs().unwrap();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn list_plugin_dbs_excludes_internal_dbs() {
        let tmp = TempDir::new().unwrap();
        let bea_dir = tmp.path().join(".bea");
        std::fs::create_dir_all(bea_dir.join("data")).unwrap();

        // Create an event log (internal _events.db).
        let mut log = EventLog::open(&bea_dir).unwrap();
        log.log("test", "cmd", &[], "ok", 1).unwrap();
        drop(log);

        let db = PluginDb::new(&bea_dir);
        let names = db.list_plugin_dbs().unwrap();
        // _events.db should NOT be listed.
        assert!(names.is_empty());
    }

    #[test]
    fn plugin_log_write_and_query() {
        let tmp = TempDir::new().unwrap();
        let bea_dir = tmp.path().join(".bea");
        std::fs::create_dir_all(bea_dir.join("data")).unwrap();

        let mut log = PluginLog::open(&bea_dir).unwrap();
        log.write("journal", "info", "opened session abc").unwrap();
        log.write("track", "info", "added artifact doc").unwrap();
        log.write("journal", "warn", "something odd").unwrap();

        // All entries.
        let entries = log.recent(10, None, None).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].plugin, "journal");
        assert_eq!(entries[0].level, "warn");

        // Filter by plugin.
        let entries = log.recent(10, Some("track"), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "added artifact doc");

        // Filter by level.
        let entries = log.recent(10, None, Some("warn")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].plugin, "journal");

        // Filter by both.
        let entries = log.recent(10, Some("journal"), Some("info")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "opened session abc");
    }

    #[test]
    fn export_and_import_roundtrip() {
        let (_tmp, mut db) = setup();

        // Create table and data.
        db.exec(
            "roundtrip",
            &DbExecRequest {
                sql: "CREATE TABLE items (name TEXT PRIMARY KEY, val TEXT)".into(),
                params: vec![],
            },
        )
        .unwrap();
        db.exec(
            "roundtrip",
            &DbExecRequest {
                sql: "INSERT INTO items (name, val) VALUES (?, ?)".into(),
                params: vec![serde_json::json!("key1"), serde_json::json!("val1")],
            },
        )
        .unwrap();

        // Export.
        let exported = db.export_plugin("roundtrip").unwrap();
        assert!(exported["items"].is_array());
        assert_eq!(exported["items"].as_array().unwrap().len(), 1);

        // Reset.
        let tables = db.reset_plugin("roundtrip").unwrap();
        assert_eq!(tables, 1);

        // Import.
        let (tables_imported, rows_imported) = db.import_plugin("roundtrip", &exported).unwrap();
        assert_eq!(tables_imported, 1);
        assert_eq!(rows_imported, 1);

        // Verify data is back.
        let res = db
            .exec(
                "roundtrip",
                &DbExecRequest {
                    sql: "SELECT name, val FROM items".into(),
                    params: vec![],
                },
            )
            .unwrap();
        assert_eq!(res.rows.len(), 1);
        assert_eq!(res.rows[0][0], serde_json::json!("key1"));
        assert_eq!(res.rows[0][1], serde_json::json!("val1"));
    }
}
