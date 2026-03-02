//! SQLite-backed implementations of the per-module store traits.
//! All data lives in a single `data/beu.db` file.

mod artifact;
mod debug;
mod event_log;
mod idea;
mod journal;
mod project;
mod state;
mod task;

use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

/// Single struct that implements all 7 store traits.
/// Command files receive `&mut impl XxxStore` and never see this type directly.
/// The `project_id` field scopes all data operations to a specific project.
pub struct SqliteStore {
    conn: Connection,
    beu_dir: PathBuf,
    project_id: String,
}

impl SqliteStore {
    pub fn open(beu_dir: &Path, project_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = beu_dir.join("data");
        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("beu.db");
        let mut conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let tx = conn.transaction()?;
        project::create_tables(&tx)?;
        artifact::create_tables(&tx)?;
        task::create_tables(&tx)?;
        journal::create_tables(&tx)?;
        state::create_tables(&tx)?;
        idea::create_tables(&tx)?;
        debug::create_tables(&tx)?;
        event_log::create_tables(&tx)?;
        tx.commit()?;

        Ok(Self {
            conn,
            beu_dir: beu_dir.to_path_buf(),
            project_id: project_id.to_string(),
        })
    }

    /// Open an existing beu database in read-only mode.
    /// Does NOT create tables or directories. Fails if the DB does not exist.
    pub fn open_readonly(
        beu_dir: &Path,
        project_id: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = beu_dir.join("data/beu.db");
        if !db_path.exists() {
            return Err(format!("database not found at {}", db_path.display()).into());
        }

        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(&db_path, flags)?;

        Ok(Self {
            conn,
            beu_dir: beu_dir.to_path_buf(),
            project_id: project_id.to_string(),
        })
    }

    pub fn beu_dir(&self) -> &Path {
        &self.beu_dir
    }

    #[cfg(test)]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
}

// ---------------------------------------------------------------------------
// Migration helpers
// ---------------------------------------------------------------------------

/// Idempotent migration: add project_id column if it does not exist.
pub(crate) fn migrate_add_project_id(
    tx: &rusqlite::Transaction,
    table: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let has_column: bool = {
        let mut stmt = tx.prepare(&format!("PRAGMA table_info([{table}])"))?;
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        columns.iter().any(|c| c == "project_id")
    };
    if !has_column {
        tx.execute_batch(&format!(
            "ALTER TABLE [{table}] ADD COLUMN project_id TEXT NOT NULL DEFAULT 'default'"
        ))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Admin operations
// ---------------------------------------------------------------------------

impl SqliteStore {
    /// Returns the known tables for a module name.
    fn module_tables(module: &str) -> Option<&'static [&'static str]> {
        match module {
            "artifact" => Some(&["artifacts", "artifact_changelog"]),
            "task" => Some(&["tasks"]),
            "journal" => Some(&["journal_sessions", "journal_entries"]),
            "state" => Some(&["state_entries"]),
            "idea" => Some(&["ideas"]),
            "debug" => Some(&["debug_sessions", "debug_entries"]),
            _ => None,
        }
    }

    /// All known module names.
    pub fn list_modules() -> &'static [&'static str] {
        &["artifact", "task", "journal", "state", "idea", "debug"]
    }

    /// Size of the beu.db file in bytes.
    pub fn db_size(&self) -> Option<u64> {
        let db_path = self.beu_dir.join("data/beu.db");
        std::fs::metadata(db_path).ok().map(|m| m.len())
    }

    /// Export a module's tables as JSON.
    pub fn export_module(
        &mut self,
        module: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let tables =
            Self::module_tables(module).ok_or_else(|| format!("unknown module '{module}'"))?;

        let tx = self.conn.transaction()?;
        let mut result = serde_json::Map::new();

        for table_name in tables {
            let sql = format!("SELECT * FROM [{table_name}] WHERE project_id = ?1");
            let mut stmt = tx.prepare(&sql)?;
            let column_count = stmt.column_count();
            let columns: Vec<String> = (0..column_count)
                .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                .collect();

            let rows: Vec<serde_json::Value> = stmt
                .query_map(rusqlite::params![self.project_id], |row| {
                    let mut obj = serde_json::Map::new();
                    for (i, col) in columns.iter().enumerate() {
                        obj.insert(col.clone(), sqlite_value_to_json(row, i));
                    }
                    Ok(serde_json::Value::Object(obj))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            result.insert(table_name.to_string(), serde_json::Value::Array(rows));
        }

        tx.commit()?;
        Ok(serde_json::Value::Object(result))
    }

    /// Import JSON data into a module's tables.
    pub fn import_module(
        &mut self,
        module: &str,
        data: &serde_json::Value,
    ) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        let known_tables =
            Self::module_tables(module).ok_or_else(|| format!("unknown module '{module}'"))?;

        let obj = data
            .as_object()
            .ok_or("import data must be a JSON object with table names as keys")?;

        let tx = self.conn.transaction()?;
        let mut tables_imported = 0usize;
        let mut rows_imported = 0usize;

        // Import tables in the order defined by module_tables() (parent tables first)
        // to satisfy foreign key constraints.
        for table_name in known_tables {
            let rows_val = match obj.get(*table_name) {
                Some(v) => v,
                None => continue,
            };
            let rows = rows_val
                .as_array()
                .ok_or_else(|| format!("table '{table_name}' value must be an array"))?;

            if rows.is_empty() {
                continue;
            }

            let first_row = rows[0]
                .as_object()
                .ok_or_else(|| format!("rows in '{table_name}' must be objects"))?;
            let columns: Vec<&String> = first_row.keys().collect();

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

    /// Delete all data for the current project in a module's tables.
    pub fn reset_module(&mut self, module: &str) -> Result<usize, Box<dyn std::error::Error>> {
        let tables =
            Self::module_tables(module).ok_or_else(|| format!("unknown module '{module}'"))?;
        let count = tables.len();

        // Delete in reverse order to respect foreign keys.
        let tx = self.conn.transaction()?;
        for table_name in tables.iter().rev() {
            tx.execute(
                &format!("DELETE FROM [{table_name}] WHERE project_id = ?1"),
                rusqlite::params![self.project_id],
            )?;
        }
        tx.commit()?;

        Ok(count)
    }

    /// Run SQLite integrity check.
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if result != "ok" {
            return Err(format!("integrity check failed: {result}").into());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// JSON <-> SQLite helpers (for admin operations)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SqliteStore) {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        let store = SqliteStore::open(&beu_dir, "default").unwrap();
        (tmp, store)
    }

    #[test]
    fn open_creates_data_dir() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        assert!(!beu_dir.join("data").exists());

        let _store = SqliteStore::open(&beu_dir, "default").unwrap();
        assert!(beu_dir.join("data/beu.db").exists());
    }

    #[test]
    fn db_size_returns_some() {
        let (_tmp, store) = setup();
        let size = store.db_size();
        assert!(size.is_some());
        assert!(size.unwrap() > 0);
    }

    #[test]
    fn export_and_import_roundtrip() {
        let (_tmp, mut store) = setup();

        // Insert test data via the artifact store.
        use crate::store::ArtifactStore;
        store.add_artifact("readme", "doc", None).unwrap();

        let exported = store.export_module("artifact").unwrap();
        let obj = exported.as_object().unwrap();
        assert!(obj.contains_key("artifacts"));
        assert_eq!(obj["artifacts"].as_array().unwrap().len(), 1);

        // Reset and re-import.
        store.reset_module("artifact").unwrap();

        // Verify empty after reset.
        let artifacts = store.list_artifacts(None).unwrap();
        assert!(artifacts.is_empty());

        let (tables, rows) = store.import_module("artifact", &exported).unwrap();
        assert_eq!(tables, 1);
        assert_eq!(rows, 1);

        // Verify data restored.
        let artifacts = store.list_artifacts(None).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].name, "readme");
    }

    #[test]
    fn export_unknown_module() {
        let (_tmp, mut store) = setup();
        let err = store.export_module("nonexistent").unwrap_err();
        assert!(err.to_string().contains("unknown module"));
    }

    #[test]
    fn reset_module_drops_and_recreates() {
        let (_tmp, mut store) = setup();

        use crate::store::TaskStore;
        store.add_task("task1", "medium", None).unwrap();

        let dropped = store.reset_module("task").unwrap();
        assert_eq!(dropped, 1);

        // Table recreated -- can add again.
        store.add_task("task2", "medium", None).unwrap();
        let tasks = store.list_tasks(None, None, None).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "task2");
    }

    #[test]
    fn reset_unknown_module() {
        let (_tmp, mut store) = setup();
        let err = store.reset_module("nonexistent").unwrap_err();
        assert!(err.to_string().contains("unknown module"));
    }

    #[test]
    fn import_invalid_data_not_object() {
        let (_tmp, mut store) = setup();
        let data = serde_json::json!("not an object");
        let err = store.import_module("task", &data).unwrap_err();
        assert!(err.to_string().contains("JSON object"));
    }

    #[test]
    fn import_invalid_data_rows_not_array() {
        let (_tmp, mut store) = setup();
        let data = serde_json::json!({"tasks": "not an array"});
        let err = store.import_module("task", &data).unwrap_err();
        assert!(err.to_string().contains("must be an array"));
    }

    #[test]
    fn import_empty_table_skipped() {
        let (_tmp, mut store) = setup();
        let data = serde_json::json!({"tasks": []});
        let (tables, rows) = store.import_module("task", &data).unwrap();
        assert_eq!(tables, 0);
        assert_eq!(rows, 0);
    }

    #[test]
    fn validate_passes() {
        let (_tmp, store) = setup();
        store.validate().unwrap();
    }

    #[test]
    fn open_readonly_succeeds() {
        let (tmp, _store) = setup();
        let beu_dir = tmp.path().join(".beu");
        drop(_store);
        let store = SqliteStore::open_readonly(&beu_dir, "default").unwrap();
        assert!(store.db_size().is_some());
    }

    #[test]
    fn open_readonly_missing_db_fails() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        let result = SqliteStore::open_readonly(&beu_dir, "default");
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("not found"));
    }

    #[test]
    fn open_readonly_can_read_data() {
        let (tmp, mut store) = setup();
        use crate::store::TaskStore;
        store.add_task("test-task", "medium", None).unwrap();
        drop(store);

        let beu_dir = tmp.path().join(".beu");
        let mut ro_store = SqliteStore::open_readonly(&beu_dir, "default").unwrap();
        let tasks = ro_store.list_tasks(None, None, None).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "test-task");
    }

    #[test]
    fn project_id_accessor() {
        let (_tmp, store) = setup();
        assert_eq!(store.project_id(), "default");
    }

    #[test]
    fn project_isolation_tasks() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");

        use crate::store::TaskStore;
        let mut store_a = SqliteStore::open(&beu_dir, "alpha").unwrap();
        store_a.add_task("alpha-task", "high", None).unwrap();
        drop(store_a);

        let mut store_b = SqliteStore::open(&beu_dir, "beta").unwrap();
        store_b.add_task("beta-task", "low", None).unwrap();

        let tasks_b = store_b.list_tasks(None, None, None).unwrap();
        assert_eq!(tasks_b.len(), 1);
        assert_eq!(tasks_b[0].title, "beta-task");
        drop(store_b);

        let mut store_a = SqliteStore::open(&beu_dir, "alpha").unwrap();
        let tasks_a = store_a.list_tasks(None, None, None).unwrap();
        assert_eq!(tasks_a.len(), 1);
        assert_eq!(tasks_a[0].title, "alpha-task");
    }

    #[test]
    fn project_isolation_artifacts() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");

        use crate::store::ArtifactStore;
        let mut store_a = SqliteStore::open(&beu_dir, "alpha").unwrap();
        store_a.add_artifact("design", "doc", None).unwrap();
        drop(store_a);

        let mut store_b = SqliteStore::open(&beu_dir, "beta").unwrap();
        let artifacts = store_b.list_artifacts(None).unwrap();
        assert!(artifacts.is_empty());

        // Both projects can have an artifact with the same name.
        store_b.add_artifact("design", "doc", None).unwrap();
        let artifacts = store_b.list_artifacts(None).unwrap();
        assert_eq!(artifacts.len(), 1);
    }

    #[test]
    fn export_module_is_project_scoped() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");

        use crate::store::ArtifactStore;
        let mut store_a = SqliteStore::open(&beu_dir, "alpha").unwrap();
        store_a.add_artifact("alpha-doc", "doc", None).unwrap();
        drop(store_a);

        let mut store_b = SqliteStore::open(&beu_dir, "beta").unwrap();
        store_b.add_artifact("beta-doc", "doc", None).unwrap();

        let exported = store_b.export_module("artifact").unwrap();
        let artifacts = exported["artifacts"].as_array().unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0]["name"], "beta-doc");
    }

    #[test]
    fn reset_module_is_project_scoped() {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");

        use crate::store::TaskStore;
        let mut store_a = SqliteStore::open(&beu_dir, "alpha").unwrap();
        store_a.add_task("alpha-task", "medium", None).unwrap();
        drop(store_a);

        let mut store_b = SqliteStore::open(&beu_dir, "beta").unwrap();
        store_b.add_task("beta-task", "medium", None).unwrap();
        store_b.reset_module("task").unwrap();

        let tasks_b = store_b.list_tasks(None, None, None).unwrap();
        assert!(tasks_b.is_empty());
        drop(store_b);

        // Alpha's data should still exist.
        let mut store_a = SqliteStore::open(&beu_dir, "alpha").unwrap();
        let tasks_a = store_a.list_tasks(None, None, None).unwrap();
        assert_eq!(tasks_a.len(), 1);
    }
}
