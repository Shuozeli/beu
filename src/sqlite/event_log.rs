use crate::store::{Event, EventLogStore};
use crate::time_helper;

use super::SqliteStore;

pub(super) fn create_tables(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            module TEXT NOT NULL,
            command TEXT NOT NULL,
            args TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL,
            duration_ms INTEGER NOT NULL DEFAULT 0,
            project_id TEXT NOT NULL DEFAULT 'default'
        );
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
        CREATE INDEX IF NOT EXISTS idx_events_module ON events(module);",
    )?;
    super::migrate_add_project_id(tx, "events")?;
    Ok(())
}

impl EventLogStore for SqliteStore {
    fn log_event(
        &mut self,
        module: &str,
        command: &str,
        args: &str,
        status: &str,
        duration_ms: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO events (timestamp, module, command, args, status, duration_ms, project_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![timestamp, module, command, args, status, duration_ms, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn recent_events(
        &mut self,
        limit: usize,
        module_filter: Option<&str>,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let events = if let Some(module) = module_filter {
            let mut stmt = tx.prepare(
                "SELECT id, timestamp, module, command, args, status, duration_ms \
                 FROM events WHERE module = ?1 AND project_id = ?2 ORDER BY id DESC LIMIT ?3",
            )?;
            let result = stmt
                .query_map(
                    rusqlite::params![module, self.project_id, limit as i64],
                    row_to_event,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            result
        } else {
            let mut stmt = tx.prepare(
                "SELECT id, timestamp, module, command, args, status, duration_ms \
                 FROM events WHERE project_id = ?1 ORDER BY id DESC LIMIT ?2",
            )?;
            let result = stmt
                .query_map(
                    rusqlite::params![self.project_id, limit as i64],
                    row_to_event,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(events)
    }

    fn count_events(&mut self) -> Result<i64, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM events WHERE project_id = ?1",
            rusqlite::params![self.project_id],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(count)
    }

    fn count_mutation_events_since(
        &mut self,
        since: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM events \
             WHERE project_id = ?1 \
               AND timestamp > ?2 \
               AND status = 'ok' \
               AND module NOT IN ('artifact', 'system') \
               AND command NOT IN ('list', 'show', 'get', 'history', 'summary')",
            rusqlite::params![self.project_id, since],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(count)
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        module: row.get(2)?,
        command: row.get(3)?,
        args: row.get(4)?,
        status: row.get(5)?,
        duration_ms: row.get(6)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::SqliteStore;
    use crate::store::EventLogStore;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SqliteStore) {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        let store = SqliteStore::open(&beu_dir, "default").unwrap();
        (tmp, store)
    }

    #[test]
    fn log_and_query() {
        let (_tmp, mut store) = setup();
        store.log_event("journal", "open", "", "ok", 12).unwrap();
        store.log_event("task", "add", "task one", "ok", 8).unwrap();

        let events = store.recent_events(10, None).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].module, "task");
        assert_eq!(events[1].command, "open");
    }

    #[test]
    fn count() {
        let (_tmp, mut store) = setup();
        assert_eq!(store.count_events().unwrap(), 0);

        store.log_event("journal", "open", "", "ok", 5).unwrap();
        assert_eq!(store.count_events().unwrap(), 1);

        store.log_event("artifact", "add", "doc", "ok", 3).unwrap();
        store.log_event("task", "add", "task", "ok", 7).unwrap();
        assert_eq!(store.count_events().unwrap(), 3);
    }

    #[test]
    fn filter_by_module() {
        let (_tmp, mut store) = setup();
        store.log_event("journal", "open", "", "ok", 5).unwrap();
        store.log_event("task", "add", "task", "ok", 3).unwrap();
        store.log_event("journal", "log", "msg", "ok", 2).unwrap();

        let journal = store.recent_events(10, Some("journal")).unwrap();
        assert_eq!(journal.len(), 2);
        assert!(journal.iter().all(|e| e.module == "journal"));

        let task = store.recent_events(10, Some("task")).unwrap();
        assert_eq!(task.len(), 1);
    }

    #[test]
    fn respects_limit() {
        let (_tmp, mut store) = setup();
        for i in 0..10 {
            store
                .log_event("journal", &format!("cmd{i}"), "", "ok", 1)
                .unwrap();
        }

        let events = store.recent_events(3, None).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].command, "cmd9");
    }

    #[test]
    fn records_error_status() {
        let (_tmp, mut store) = setup();
        store
            .log_event("journal", "log", "msg", "error", 0)
            .unwrap();

        let events = store.recent_events(1, None).unwrap();
        assert_eq!(events[0].status, "error");
    }

    #[test]
    fn count_mutation_events_since_excludes_reads() {
        let (_tmp, mut store) = setup();
        let since = "2020-01-01T00:00:00.000Z";

        // Mutation events (should count)
        store.log_event("task", "add", "task one", "ok", 5).unwrap();
        store.log_event("task", "done", "1", "ok", 3).unwrap();
        store.log_event("state", "set", "key val", "ok", 2).unwrap();

        // Read-only events (should NOT count)
        store.log_event("task", "list", "", "ok", 1).unwrap();
        store.log_event("task", "show", "1", "ok", 1).unwrap();
        store.log_event("state", "get", "key", "ok", 1).unwrap();
        store.log_event("idea", "list", "", "ok", 1).unwrap();
        store.log_event("debug", "show", "slug", "ok", 1).unwrap();

        let count = store.count_mutation_events_since(since).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn count_mutation_events_since_excludes_artifact_module() {
        let (_tmp, mut store) = setup();
        let since = "2020-01-01T00:00:00.000Z";

        // Artifact events (should NOT count -- doc updates are not "changes")
        store
            .log_event("artifact", "add", "design", "ok", 3)
            .unwrap();
        store
            .log_event("artifact", "status", "design done", "ok", 2)
            .unwrap();
        store
            .log_event("artifact", "changelog", "design msg", "ok", 1)
            .unwrap();

        // System events (should NOT count)
        store.log_event("system", "check", "", "ok", 1).unwrap();
        store.log_event("system", "pause", "msg", "ok", 1).unwrap();

        // One real mutation
        store.log_event("task", "add", "task", "ok", 5).unwrap();

        let count = store.count_mutation_events_since(since).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn count_mutation_events_since_respects_timestamp() {
        let (_tmp, mut store) = setup();

        // Log some events (timestamps are auto-generated as "now")
        store.log_event("task", "add", "task one", "ok", 5).unwrap();
        store.log_event("task", "done", "1", "ok", 3).unwrap();

        // Use a future timestamp -- nothing should be after it
        let future = "2099-01-01T00:00:00.000Z";
        let count = store.count_mutation_events_since(future).unwrap();
        assert_eq!(count, 0);

        // Use a past timestamp -- both should count
        let past = "2020-01-01T00:00:00.000Z";
        let count = store.count_mutation_events_since(past).unwrap();
        assert_eq!(count, 2);
    }
}
