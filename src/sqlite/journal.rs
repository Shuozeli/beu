use crate::store::{JournalEntry, JournalStore, Session};
use crate::time_helper;

use super::SqliteStore;

pub(super) fn create_tables(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS journal_sessions (
            id TEXT PRIMARY KEY,
            started_at TEXT NOT NULL,
            closed_at TEXT,
            status TEXT NOT NULL DEFAULT 'open',
            project_id TEXT NOT NULL DEFAULT 'default'
        );
        CREATE TABLE IF NOT EXISTS journal_entries (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            message TEXT NOT NULL,
            tag TEXT,
            project_id TEXT NOT NULL DEFAULT 'default',
            FOREIGN KEY (session_id) REFERENCES journal_sessions(id)
        );",
    )?;
    super::migrate_add_project_id(tx, "journal_sessions")?;
    super::migrate_add_project_id(tx, "journal_entries")?;
    Ok(())
}

impl JournalStore for SqliteStore {
    fn create_session(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO journal_sessions (id, started_at, status, project_id) VALUES (?1, ?2, 'open', ?3)",
            rusqlite::params![session_id, now, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn get_open_session_id(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let result = {
            let mut stmt = tx.prepare(
                "SELECT id FROM journal_sessions WHERE status = 'open' \
                 AND project_id = ?1 \
                 ORDER BY rowid DESC LIMIT 1",
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

    fn get_session(
        &mut self,
        session_id: &str,
    ) -> Result<Option<Session>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let result = {
            let mut stmt = tx.prepare(
                "SELECT id, started_at, closed_at, status FROM journal_sessions WHERE id = ?1 AND project_id = ?2",
            )?;
            let result = stmt
                .query_map(rusqlite::params![session_id, self.project_id], |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        started_at: row.get(1)?,
                        closed_at: row.get(2)?,
                        status: row.get(3)?,
                    })
                })?
                .next()
                .transpose()?;
            result
        };
        tx.commit()?;
        Ok(result)
    }

    fn close_session(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE journal_sessions SET status = 'closed', closed_at = ?1 WHERE id = ?2 AND project_id = ?3",
            rusqlite::params![now, session_id, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn add_entry(
        &mut self,
        session_id: &str,
        entry_id: &str,
        message: &str,
        tag: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO journal_entries (id, session_id, created_at, message, tag, project_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![entry_id, session_id, now, message, tag, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn list_entries(
        &mut self,
        session_id: &str,
    ) -> Result<Vec<JournalEntry>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let entries = {
            let mut stmt = tx.prepare(
                "SELECT created_at, message, tag FROM journal_entries \
                 WHERE session_id = ?1 AND project_id = ?2 ORDER BY created_at",
            )?;
            let result = stmt
                .query_map(rusqlite::params![session_id, self.project_id], |row| {
                    Ok(JournalEntry {
                        created_at: row.get(0)?,
                        message: row.get(1)?,
                        tag: row.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(entries)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::SqliteStore;
    use crate::store::JournalStore;
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
        store.create_session("s-001").unwrap();

        let session = store.get_session("s-001").unwrap().unwrap();
        assert_eq!(session.id, "s-001");
        assert_eq!(session.status, "open");
        assert!(session.closed_at.is_none());
    }

    #[test]
    fn get_nonexistent_session_returns_none() {
        let (_tmp, mut store) = setup();
        assert!(store.get_session("nope").unwrap().is_none());
    }

    #[test]
    fn get_open_session_id() {
        let (_tmp, mut store) = setup();
        assert!(store.get_open_session_id().unwrap().is_none());

        store.create_session("s-001").unwrap();
        assert_eq!(
            store.get_open_session_id().unwrap(),
            Some("s-001".to_string())
        );
    }

    #[test]
    fn close_session() {
        let (_tmp, mut store) = setup();
        store.create_session("s-001").unwrap();
        store.close_session("s-001").unwrap();

        let session = store.get_session("s-001").unwrap().unwrap();
        assert_eq!(session.status, "closed");
        assert!(session.closed_at.is_some());

        assert!(store.get_open_session_id().unwrap().is_none());
    }

    #[test]
    fn add_and_list_entries() {
        let (_tmp, mut store) = setup();
        store.create_session("s-001").unwrap();
        store
            .add_entry("s-001", "e-001", "first message", None)
            .unwrap();
        store
            .add_entry("s-001", "e-002", "tagged note", Some("decision"))
            .unwrap();

        let entries = store.list_entries("s-001").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "first message");
        assert!(entries[0].tag.is_none());
        assert_eq!(entries[1].message, "tagged note");
        assert_eq!(entries[1].tag, Some("decision".to_string()));
    }

    #[test]
    fn entries_isolated_by_session() {
        let (_tmp, mut store) = setup();
        store.create_session("s-001").unwrap();
        store.create_session("s-002").unwrap();
        store
            .add_entry("s-001", "e-001", "msg for s1", None)
            .unwrap();
        store
            .add_entry("s-002", "e-002", "msg for s2", None)
            .unwrap();

        let entries_1 = store.list_entries("s-001").unwrap();
        assert_eq!(entries_1.len(), 1);
        assert_eq!(entries_1[0].message, "msg for s1");

        let entries_2 = store.list_entries("s-002").unwrap();
        assert_eq!(entries_2.len(), 1);
        assert_eq!(entries_2[0].message, "msg for s2");
    }

    #[test]
    fn most_recent_open_session_wins() {
        let (_tmp, mut store) = setup();
        store.create_session("s-001").unwrap();
        store.create_session("s-002").unwrap();

        let open = store.get_open_session_id().unwrap().unwrap();
        assert_eq!(open, "s-002");
    }
}
