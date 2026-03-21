use crate::store::{Idea, IdeaStore};
use crate::time_helper;

use super::SqliteStore;

pub(super) fn create_tables(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS ideas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            area TEXT NOT NULL DEFAULT 'general',
            description TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            priority TEXT NOT NULL DEFAULT 'medium',
            project_id TEXT NOT NULL DEFAULT 'default',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );",
    )?;
    super::migrate_add_project_id(tx, "ideas")?;
    Ok(())
}

impl IdeaStore for SqliteStore {
    fn add_idea(
        &mut self,
        title: &str,
        area: &str,
        priority: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO ideas (title, area, status, priority, project_id, created_at, updated_at) \
             VALUES (?1, ?2, 'pending', ?3, ?4, ?5, ?6)",
            rusqlite::params![title, area, priority, self.project_id, now, now],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    fn get_idea(&mut self, id: i64) -> Result<Option<Idea>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let result = {
            let mut stmt = tx.prepare(
                "SELECT id, title, area, description, status, priority, created_at, updated_at \
                 FROM ideas WHERE id = ?1 AND project_id = ?2",
            )?;
            let result = stmt
                .query_map(rusqlite::params![id, self.project_id], row_to_idea)?
                .next()
                .transpose()?;
            result
        };
        tx.commit()?;
        Ok(result)
    }

    fn list_ideas(
        &mut self,
        area: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<Idea>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let order = "ORDER BY CASE priority \
            WHEN 'high' THEN 0 WHEN 'medium' THEN 1 WHEN 'low' THEN 2 END, id";

        let ideas = match (area, status) {
            (Some(a), Some(s)) => {
                let mut stmt = tx.prepare(&format!(
                    "SELECT id, title, area, description, status, priority, created_at, updated_at \
                     FROM ideas WHERE area = ?1 AND status = ?2 AND project_id = ?3 {order}"
                ))?;
                let result = stmt
                    .query_map(rusqlite::params![a, s, self.project_id], row_to_idea)?
                    .collect::<Result<Vec<_>, _>>()?;
                result
            }
            (Some(a), None) => {
                let mut stmt = tx.prepare(&format!(
                    "SELECT id, title, area, description, status, priority, created_at, updated_at \
                     FROM ideas WHERE area = ?1 AND project_id = ?2 {order}"
                ))?;
                let result = stmt
                    .query_map(rusqlite::params![a, self.project_id], row_to_idea)?
                    .collect::<Result<Vec<_>, _>>()?;
                result
            }
            (None, Some(s)) => {
                let mut stmt = tx.prepare(&format!(
                    "SELECT id, title, area, description, status, priority, created_at, updated_at \
                     FROM ideas WHERE status = ?1 AND project_id = ?2 {order}"
                ))?;
                let result = stmt
                    .query_map(rusqlite::params![s, self.project_id], row_to_idea)?
                    .collect::<Result<Vec<_>, _>>()?;
                result
            }
            (None, None) => {
                let mut stmt = tx.prepare(&format!(
                    "SELECT id, title, area, description, status, priority, created_at, updated_at \
                     FROM ideas WHERE status != 'archived' AND project_id = ?1 {order}"
                ))?;
                let result = stmt
                    .query_map(rusqlite::params![self.project_id], row_to_idea)?
                    .collect::<Result<Vec<_>, _>>()?;
                result
            }
        };

        tx.commit()?;
        Ok(ideas)
    }

    fn set_idea_done(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let title: Option<String> = {
            let mut stmt =
                tx.prepare("SELECT title FROM ideas WHERE id = ?1 AND project_id = ?2")?;
            let result = stmt
                .query_map(rusqlite::params![id, self.project_id], |row| {
                    row.get::<_, String>(0)
                })?
                .next()
                .transpose()?;
            result
        };
        let title = title.ok_or_else(|| format!("idea #{id} not found"))?;

        let now = time_helper::utc_now();
        tx.execute(
            "UPDATE ideas SET status = 'done', updated_at = ?1 WHERE id = ?2 AND project_id = ?3",
            rusqlite::params![now, id, self.project_id],
        )?;
        tx.commit()?;
        Ok(title)
    }

    fn archive_idea(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let title: Option<String> = {
            let mut stmt =
                tx.prepare("SELECT title FROM ideas WHERE id = ?1 AND project_id = ?2")?;
            let result = stmt
                .query_map(rusqlite::params![id, self.project_id], |row| {
                    row.get::<_, String>(0)
                })?
                .next()
                .transpose()?;
            result
        };
        let title = title.ok_or_else(|| format!("idea #{id} not found"))?;

        let now = time_helper::utc_now();
        tx.execute(
            "UPDATE ideas SET status = 'archived', updated_at = ?1 WHERE id = ?2 AND project_id = ?3",
            rusqlite::params![now, id, self.project_id],
        )?;
        tx.commit()?;
        Ok(title)
    }

    fn describe_idea(
        &mut self,
        id: i64,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let exists = tx
            .prepare("SELECT 1 FROM ideas WHERE id = ?1 AND project_id = ?2")?
            .exists(rusqlite::params![id, self.project_id])?;
        if !exists {
            return Err(format!("idea #{id} not found").into());
        }

        let now = time_helper::utc_now();
        tx.execute(
            "UPDATE ideas SET description = ?1, updated_at = ?2 WHERE id = ?3 AND project_id = ?4",
            rusqlite::params![description, now, id, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn count_ideas_by_status(&mut self) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let counts = {
            let mut stmt = tx.prepare(
                "SELECT status, COUNT(*) FROM ideas WHERE project_id = ?1 GROUP BY status ORDER BY status",
            )?;
            let result = stmt
                .query_map(rusqlite::params![self.project_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(counts)
    }
}

fn row_to_idea(row: &rusqlite::Row<'_>) -> rusqlite::Result<Idea> {
    Ok(Idea {
        id: row.get(0)?,
        title: row.get(1)?,
        area: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        priority: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::SqliteStore;
    use crate::store::IdeaStore;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SqliteStore) {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        let store = SqliteStore::open(&beu_dir, "default").unwrap();
        (tmp, store)
    }

    #[test]
    fn add_and_get() {
        let (_tmp, mut store) = setup();
        let id = store.add_idea("rate limiting", "api", "high").unwrap();
        assert!(id > 0);

        let idea = store.get_idea(id).unwrap().unwrap();
        assert_eq!(idea.title, "rate limiting");
        assert_eq!(idea.area, "api");
        assert_eq!(idea.priority, "high");
        assert_eq!(idea.status, "pending");
        assert!(idea.description.is_none());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let (_tmp, mut store) = setup();
        assert!(store.get_idea(999).unwrap().is_none());
    }

    #[test]
    fn list_ideas_filters() {
        let (_tmp, mut store) = setup();
        store.add_idea("api idea", "api", "high").unwrap();
        store.add_idea("ui idea", "ui", "medium").unwrap();

        let api = store.list_ideas(Some("api"), None).unwrap();
        assert_eq!(api.len(), 1);
        assert_eq!(api[0].title, "api idea");

        let pending = store.list_ideas(None, Some("pending")).unwrap();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn list_ideas_excludes_archived_by_default() {
        let (_tmp, mut store) = setup();
        let id1 = store.add_idea("active", "general", "medium").unwrap();
        let id2 = store.add_idea("old", "general", "low").unwrap();
        store.archive_idea(id2).unwrap();

        let ideas = store.list_ideas(None, None).unwrap();
        assert_eq!(ideas.len(), 1);
        assert_eq!(ideas[0].id, id1);
    }

    #[test]
    fn set_idea_done() {
        let (_tmp, mut store) = setup();
        let id = store.add_idea("finish docs", "docs", "medium").unwrap();

        let title = store.set_idea_done(id).unwrap();
        assert_eq!(title, "finish docs");

        let idea = store.get_idea(id).unwrap().unwrap();
        assert_eq!(idea.status, "done");
    }

    #[test]
    fn set_done_not_found() {
        let (_tmp, mut store) = setup();
        let err = store.set_idea_done(999).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn archive_idea() {
        let (_tmp, mut store) = setup();
        let id = store.add_idea("old idea", "general", "low").unwrap();

        let title = store.archive_idea(id).unwrap();
        assert_eq!(title, "old idea");

        let idea = store.get_idea(id).unwrap().unwrap();
        assert_eq!(idea.status, "archived");
    }

    #[test]
    fn describe_idea() {
        let (_tmp, mut store) = setup();
        let id = store.add_idea("design api", "api", "high").unwrap();
        store.describe_idea(id, "Detailed design doc").unwrap();

        let idea = store.get_idea(id).unwrap().unwrap();
        assert_eq!(idea.description, Some("Detailed design doc".to_string()));
    }

    #[test]
    fn describe_not_found() {
        let (_tmp, mut store) = setup();
        let err = store.describe_idea(999, "desc").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn count_ideas_by_status() {
        let (_tmp, mut store) = setup();
        store.add_idea("a", "general", "medium").unwrap();
        store.add_idea("b", "general", "medium").unwrap();
        let id3 = store.add_idea("c", "general", "medium").unwrap();
        store.set_idea_done(id3).unwrap();

        let counts = store.count_ideas_by_status().unwrap();
        let pending = counts
            .iter()
            .find(|(s, _)| s == "pending")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let done = counts
            .iter()
            .find(|(s, _)| s == "done")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        assert_eq!(pending, 2);
        assert_eq!(done, 1);
    }

    #[test]
    fn priority_ordering() {
        let (_tmp, mut store) = setup();
        store.add_idea("low idea", "general", "low").unwrap();
        store.add_idea("high idea", "general", "high").unwrap();
        store.add_idea("medium idea", "general", "medium").unwrap();

        let ideas = store.list_ideas(None, None).unwrap();
        assert_eq!(ideas[0].priority, "high");
        assert_eq!(ideas[1].priority, "medium");
        assert_eq!(ideas[2].priority, "low");
    }
}
