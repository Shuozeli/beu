use crate::store::{DebugEntry, DebugSession, DebugStore};
use crate::time_helper;

use super::SqliteStore;

pub(super) fn create_tables(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS debug_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            slug TEXT NOT NULL,
            title TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'investigating',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            project_id TEXT NOT NULL DEFAULT 'default',
            UNIQUE(slug, project_id)
        );
        CREATE TABLE IF NOT EXISTS debug_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_slug TEXT NOT NULL,
            entry_type TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            project_id TEXT NOT NULL DEFAULT 'default',
            FOREIGN KEY (project_id, session_slug) REFERENCES debug_sessions(project_id, slug)
        );",
    )?;
    super::migrate_add_project_id(tx, "debug_sessions")?;
    super::migrate_add_project_id(tx, "debug_entries")?;
    Ok(())
}

impl DebugStore for SqliteStore {
    fn create_session(
        &mut self,
        slug: &str,
        title: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO debug_sessions (slug, title, status, created_at, updated_at, project_id) \
             VALUES (?1, ?2, 'investigating', ?3, ?4, ?5)",
            rusqlite::params![slug, title, now, now, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn slug_exists(&mut self, slug: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let exists = tx
            .prepare("SELECT 1 FROM debug_sessions WHERE slug = ?1 AND project_id = ?2")?
            .exists(rusqlite::params![slug, self.project_id])?;
        tx.commit()?;
        Ok(exists)
    }

    fn get_session(
        &mut self,
        slug: &str,
    ) -> Result<Option<DebugSession>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let result = {
            let mut stmt = tx.prepare(
                "SELECT slug, title, status, created_at, updated_at \
                 FROM debug_sessions WHERE slug = ?1 AND project_id = ?2",
            )?;
            let result = stmt
                .query_map(rusqlite::params![slug, self.project_id], row_to_session)?
                .next()
                .transpose()?;
            result
        };
        tx.commit()?;
        Ok(result)
    }

    fn add_entry(
        &mut self,
        slug: &str,
        entry_type: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO debug_entries (session_slug, entry_type, content, created_at, project_id) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![slug, entry_type, content, now, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn update_status(
        &mut self,
        slug: &str,
        status: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE debug_sessions SET status = ?1, updated_at = ?2 WHERE slug = ?3 AND project_id = ?4",
            rusqlite::params![status, now, slug, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn update_timestamp(&mut self, slug: &str) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE debug_sessions SET updated_at = ?1 WHERE slug = ?2 AND project_id = ?3",
            rusqlite::params![now, slug, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn list_sessions(
        &mut self,
        status_filter: Option<&str>,
    ) -> Result<Vec<DebugSession>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let sessions = if let Some(s) = status_filter {
            let mut stmt = tx.prepare(
                "SELECT slug, title, status, created_at, updated_at \
                 FROM debug_sessions WHERE status = ?1 AND project_id = ?2 ORDER BY rowid DESC",
            )?;
            let result = stmt
                .query_map(rusqlite::params![s, self.project_id], row_to_session)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        } else {
            let mut stmt = tx.prepare(
                "SELECT slug, title, status, created_at, updated_at \
                 FROM debug_sessions WHERE project_id = ?1 ORDER BY rowid DESC",
            )?;
            let result = stmt
                .query_map(rusqlite::params![self.project_id], row_to_session)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };

        tx.commit()?;
        Ok(sessions)
    }

    fn list_entries(&mut self, slug: &str) -> Result<Vec<DebugEntry>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let entries = {
            let mut stmt = tx.prepare(
                "SELECT entry_type, content, created_at FROM debug_entries \
                 WHERE session_slug = ?1 AND project_id = ?2 ORDER BY id ASC",
            )?;
            let result = stmt
                .query_map(rusqlite::params![slug, self.project_id], |row| {
                    Ok(DebugEntry {
                        entry_type: row.get(0)?,
                        content: row.get(1)?,
                        created_at: row.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(entries)
    }

    fn count_active(&mut self) -> Result<i64, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM debug_sessions WHERE status != 'resolved' AND project_id = ?1",
            rusqlite::params![self.project_id],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(count)
    }
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<DebugSession> {
    Ok(DebugSession {
        slug: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
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
    use crate::store::DebugStore;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SqliteStore) {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        let store = SqliteStore::open(&beu_dir, "default").unwrap();
        (tmp, store)
    }

    #[test]
    fn create_and_get_session() {
        let (_tmp, mut store) = setup();
        store
            .create_session("null-ptr", "Null pointer crash")
            .unwrap();

        let session = store.get_session("null-ptr").unwrap().unwrap();
        assert_eq!(session.slug, "null-ptr");
        assert_eq!(session.title, "Null pointer crash");
        assert_eq!(session.status, "investigating");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let (_tmp, mut store) = setup();
        assert!(store.get_session("nope").unwrap().is_none());
    }

    #[test]
    fn slug_exists() {
        let (_tmp, mut store) = setup();
        assert!(!store.slug_exists("bug").unwrap());

        store.create_session("bug", "A bug").unwrap();
        assert!(store.slug_exists("bug").unwrap());
    }

    #[test]
    fn add_and_list_entries() {
        let (_tmp, mut store) = setup();
        store.create_session("crash", "App crash").unwrap();
        store
            .add_entry("crash", "symptom", "segfault on startup")
            .unwrap();
        store
            .add_entry("crash", "evidence", "core dump shows null deref")
            .unwrap();

        let entries = store.list_entries("crash").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry_type, "symptom");
        assert_eq!(entries[1].entry_type, "evidence");
    }

    #[test]
    fn entries_isolated_by_session() {
        let (_tmp, mut store) = setup();
        store.create_session("bug-a", "Bug A").unwrap();
        store.create_session("bug-b", "Bug B").unwrap();
        store.add_entry("bug-a", "symptom", "for A").unwrap();
        store.add_entry("bug-b", "symptom", "for B").unwrap();

        assert_eq!(store.list_entries("bug-a").unwrap().len(), 1);
        assert_eq!(store.list_entries("bug-b").unwrap().len(), 1);
    }

    #[test]
    fn update_status() {
        let (_tmp, mut store) = setup();
        store.create_session("issue", "An issue").unwrap();
        store.update_status("issue", "root-cause-found").unwrap();

        let session = store.get_session("issue").unwrap().unwrap();
        assert_eq!(session.status, "root-cause-found");
    }

    #[test]
    fn update_timestamp() {
        let (_tmp, mut store) = setup();
        store.create_session("issue", "An issue").unwrap();
        let before = store.get_session("issue").unwrap().unwrap().updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        store.update_timestamp("issue").unwrap();

        let after = store.get_session("issue").unwrap().unwrap().updated_at;
        assert!(after >= before);
    }

    #[test]
    fn list_sessions_all() {
        let (_tmp, mut store) = setup();
        store.create_session("a", "Session A").unwrap();
        store.create_session("b", "Session B").unwrap();

        let all = store.list_sessions(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn list_sessions_with_filter() {
        let (_tmp, mut store) = setup();
        store.create_session("open-bug", "Open bug").unwrap();
        store.create_session("fixed-bug", "Fixed bug").unwrap();
        store.update_status("fixed-bug", "resolved").unwrap();

        let investigating = store.list_sessions(Some("investigating")).unwrap();
        assert_eq!(investigating.len(), 1);
        assert_eq!(investigating[0].slug, "open-bug");

        let resolved = store.list_sessions(Some("resolved")).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].slug, "fixed-bug");
    }

    #[test]
    fn count_active() {
        let (_tmp, mut store) = setup();
        store.create_session("a", "A").unwrap();
        store.create_session("b", "B").unwrap();
        store.create_session("c", "C").unwrap();

        assert_eq!(store.count_active().unwrap(), 3);

        store.update_status("b", "resolved").unwrap();
        assert_eq!(store.count_active().unwrap(), 2);
    }

    #[test]
    fn duplicate_slug_fails() {
        let (_tmp, mut store) = setup();
        store.create_session("dup", "First").unwrap();
        let err = store.create_session("dup", "Second").unwrap_err();
        assert!(err.to_string().contains("UNIQUE") || err.to_string().contains("constraint"));
    }
}
