use crate::store::{StateEntry, StateStore};
use crate::time_helper;

use super::SqliteStore;

pub(super) fn create_tables(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS state_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            project_id TEXT NOT NULL DEFAULT 'default',
            UNIQUE(project_id, category, key)
        );",
    )?;
    super::migrate_add_project_id(tx, "state_entries")?;
    Ok(())
}

impl StateStore for SqliteStore {
    fn set(
        &mut self,
        category: &str,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO state_entries (category, key, value, created_at, updated_at, project_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(project_id, category, key) DO UPDATE SET \
             value=excluded.value, updated_at=excluded.updated_at",
            rusqlite::params![category, key, value, now, now, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn get_by_key(&mut self, key: &str) -> Result<Option<StateEntry>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let result = {
            let mut stmt = tx.prepare(
                "SELECT category, key, value, created_at, updated_at \
                 FROM state_entries WHERE key = ?1 AND project_id = ?2",
            )?;
            let result = stmt
                .query_map(rusqlite::params![key, self.project_id], row_to_state_entry)?
                .next()
                .transpose()?;
            result
        };
        tx.commit()?;
        Ok(result)
    }

    fn list_entries(
        &mut self,
        category: Option<&str>,
    ) -> Result<Vec<StateEntry>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let entries = if let Some(cat) = category {
            let mut stmt = tx.prepare(
                "SELECT category, key, value, created_at, updated_at \
                 FROM state_entries WHERE category = ?1 AND project_id = ?2 ORDER BY key",
            )?;
            let result = stmt
                .query_map(rusqlite::params![cat, self.project_id], row_to_state_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        } else {
            let mut stmt = tx.prepare(
                "SELECT category, key, value, created_at, updated_at \
                 FROM state_entries WHERE project_id = ?1 ORDER BY category, key",
            )?;
            let result = stmt
                .query_map(rusqlite::params![self.project_id], row_to_state_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };

        tx.commit()?;
        Ok(entries)
    }

    fn remove(&mut self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let exists = {
            let exists = tx
                .prepare("SELECT 1 FROM state_entries WHERE key = ?1 AND project_id = ?2")?
                .exists(rusqlite::params![key, self.project_id])?;
            exists
        };
        if !exists {
            tx.commit()?;
            return Ok(false);
        }

        tx.execute(
            "DELETE FROM state_entries WHERE key = ?1 AND project_id = ?2",
            rusqlite::params![key, self.project_id],
        )?;
        tx.commit()?;
        Ok(true)
    }

    fn clear_category(&mut self, category: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let count = tx.execute(
            "DELETE FROM state_entries WHERE category = ?1 AND project_id = ?2",
            rusqlite::params![category, self.project_id],
        )?;
        tx.commit()?;
        Ok(count as u64)
    }

    fn count_by_category(&mut self, category: &str) -> Result<i64, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM state_entries WHERE category = ?1 AND project_id = ?2",
            rusqlite::params![category, self.project_id],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(count)
    }

    fn get_checkpoint(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let result = {
            let mut stmt = tx.prepare(
                "SELECT value FROM state_entries \
                 WHERE category = 'focus' AND key = '_checkpoint' AND project_id = ?1",
            )?;
            let result = stmt
                .query_map(rusqlite::params![self.project_id], |row| {
                    row.get::<_, String>(0)
                })?
                .next()
                .transpose()?;
            result
        };
        tx.commit()?;
        Ok(result)
    }

    fn clear_checkpoint(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM state_entries WHERE category = 'focus' AND key = '_checkpoint' AND project_id = ?1",
            rusqlite::params![self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn list_blockers(&mut self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let blockers = {
            let mut stmt = tx.prepare(
                "SELECT key, value FROM state_entries \
                 WHERE category = 'blocker' AND project_id = ?1 ORDER BY key",
            )?;
            let result = stmt
                .query_map(rusqlite::params![self.project_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(blockers)
    }

    fn list_focus_items(&mut self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let items = {
            let mut stmt = tx.prepare(
                "SELECT key, value FROM state_entries \
                 WHERE category = 'focus' AND key != '_checkpoint' AND project_id = ?1 ORDER BY key",
            )?;
            let result = stmt
                .query_map(rusqlite::params![self.project_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(items)
    }
}

fn row_to_state_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<StateEntry> {
    Ok(StateEntry {
        category: row.get(0)?,
        key: row.get(1)?,
        value: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::SqliteStore;
    use crate::store::StateStore;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SqliteStore) {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        let store = SqliteStore::open(&beu_dir, "default").unwrap();
        (tmp, store)
    }

    #[test]
    fn set_and_get() {
        let (_tmp, mut store) = setup();
        store.set("decision", "auth-method", "JWT tokens").unwrap();

        let entry = store.get_by_key("auth-method").unwrap().unwrap();
        assert_eq!(entry.category, "decision");
        assert_eq!(entry.key, "auth-method");
        assert_eq!(entry.value, "JWT tokens");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let (_tmp, mut store) = setup();
        assert!(store.get_by_key("nope").unwrap().is_none());
    }

    #[test]
    fn upsert_overwrites_value() {
        let (_tmp, mut store) = setup();
        store.set("decision", "db", "postgres").unwrap();
        store.set("decision", "db", "sqlite").unwrap();

        let entry = store.get_by_key("db").unwrap().unwrap();
        assert_eq!(entry.value, "sqlite");
    }

    #[test]
    fn list_entries_all() {
        let (_tmp, mut store) = setup();
        store.set("decision", "auth", "JWT").unwrap();
        store.set("blocker", "ci", "flaky tests").unwrap();

        let all = store.list_entries(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn list_entries_by_category() {
        let (_tmp, mut store) = setup();
        store.set("decision", "auth", "JWT").unwrap();
        store.set("blocker", "ci", "flaky tests").unwrap();
        store.set("decision", "db", "postgres").unwrap();

        let decisions = store.list_entries(Some("decision")).unwrap();
        assert_eq!(decisions.len(), 2);
        assert!(decisions.iter().all(|e| e.category == "decision"));
    }

    #[test]
    fn remove_entry() {
        let (_tmp, mut store) = setup();
        store.set("decision", "auth", "JWT").unwrap();

        assert!(store.remove("auth").unwrap());
        assert!(store.get_by_key("auth").unwrap().is_none());
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let (_tmp, mut store) = setup();
        assert!(!store.remove("nope").unwrap());
    }

    #[test]
    fn clear_category() {
        let (_tmp, mut store) = setup();
        store.set("blocker", "ci", "flaky tests").unwrap();
        store.set("blocker", "infra", "no staging").unwrap();
        store.set("decision", "auth", "JWT").unwrap();

        let cleared = store.clear_category("blocker").unwrap();
        assert_eq!(cleared, 2);

        assert!(store.get_by_key("auth").unwrap().is_some());
        assert!(store.get_by_key("ci").unwrap().is_none());
    }

    #[test]
    fn count_by_category() {
        let (_tmp, mut store) = setup();
        store.set("blocker", "a", "val").unwrap();
        store.set("blocker", "b", "val").unwrap();
        store.set("decision", "c", "val").unwrap();

        assert_eq!(store.count_by_category("blocker").unwrap(), 2);
        assert_eq!(store.count_by_category("decision").unwrap(), 1);
        assert_eq!(store.count_by_category("focus").unwrap(), 0);
    }

    #[test]
    fn checkpoint_lifecycle() {
        let (_tmp, mut store) = setup();
        assert!(store.get_checkpoint().unwrap().is_none());

        store
            .set("focus", "_checkpoint", "working on auth")
            .unwrap();
        assert_eq!(
            store.get_checkpoint().unwrap(),
            Some("working on auth".to_string())
        );

        store.clear_checkpoint().unwrap();
        assert!(store.get_checkpoint().unwrap().is_none());
    }

    #[test]
    fn list_blockers() {
        let (_tmp, mut store) = setup();
        store
            .set("blocker", "ci-flaky", "tests fail randomly")
            .unwrap();
        store
            .set("blocker", "no-staging", "no staging env")
            .unwrap();
        store.set("decision", "unrelated", "not a blocker").unwrap();

        let blockers = store.list_blockers().unwrap();
        assert_eq!(blockers.len(), 2);
        assert!(blockers
            .iter()
            .all(|(k, _)| k == "ci-flaky" || k == "no-staging"));
    }

    #[test]
    fn list_focus_items_excludes_checkpoint() {
        let (_tmp, mut store) = setup();
        store.set("focus", "_checkpoint", "paused state").unwrap();
        store
            .set("focus", "auth-module", "finish login flow")
            .unwrap();
        store
            .set("focus", "tests", "add integration tests")
            .unwrap();

        let focus = store.list_focus_items().unwrap();
        assert_eq!(focus.len(), 2);
        assert!(focus.iter().all(|(k, _)| k != "_checkpoint"));
    }
}
