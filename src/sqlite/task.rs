use crate::store::{Task, TaskStore};
use crate::time_helper;

use super::SqliteStore;

pub(super) fn create_tables(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'open',
            priority TEXT NOT NULL DEFAULT 'medium',
            tag TEXT,
            test_status TEXT NOT NULL DEFAULT 'planned',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            project_id TEXT NOT NULL DEFAULT 'default'
        );",
    )?;
    super::migrate_add_project_id(tx, "tasks")?;
    migrate_add_test_status(tx)?;
    Ok(())
}

fn migrate_add_test_status(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    let has_column: bool = {
        let mut stmt = tx.prepare("PRAGMA table_info([tasks])").unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        columns.iter().any(|c| c == "test_status")
    };
    if !has_column {
        tx.execute_batch(
            "ALTER TABLE [tasks] ADD COLUMN test_status TEXT NOT NULL DEFAULT 'planned'",
        )?;
    }
    Ok(())
}

impl TaskStore for SqliteStore {
    fn add_task(
        &mut self,
        title: &str,
        priority: &str,
        tag: Option<&str>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO tasks (title, status, priority, tag, test_status, created_at, updated_at, project_id) \
             VALUES (?1, 'open', ?2, ?3, 'planned', ?4, ?5, ?6)",
            rusqlite::params![title, priority, tag, now, now, self.project_id],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    fn get_task(&mut self, id: i64) -> Result<Option<Task>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let result = {
            let mut stmt = tx.prepare(
                "SELECT id, title, status, priority, tag, test_status, created_at, updated_at \
                 FROM tasks WHERE id = ?1 AND project_id = ?2",
            )?;
            let result = stmt
                .query_map(rusqlite::params![id, self.project_id], row_to_task)?
                .next()
                .transpose()?;
            result
        };
        tx.commit()?;
        Ok(result)
    }

    fn list_tasks(
        &mut self,
        status_filter: Option<&str>,
        tag_filter: Option<&str>,
        test_status_filter: Option<&str>,
    ) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let cols = "SELECT id, title, status, priority, tag, test_status, created_at, updated_at FROM tasks";
        let order = "ORDER BY CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 \
            WHEN 'medium' THEN 2 ELSE 3 END, id";

        let mut clauses = vec!["project_id = ?1".to_string()];
        let mut param_idx = 2usize;

        if status_filter.is_some() {
            clauses.push(format!("status = ?{param_idx}"));
            param_idx += 1;
        }
        if tag_filter.is_some() {
            clauses.push(format!("tag = ?{param_idx}"));
            param_idx += 1;
        }
        if test_status_filter.is_some() {
            clauses.push(format!("test_status = ?{param_idx}"));
        }

        let sql = format!("{cols} WHERE {} {order}", clauses.join(" AND "));

        // Clone all filter values upfront so nothing borrows from `self` during the query.
        let project_id = self.project_id.clone();
        let s_owned: Option<String> = status_filter.map(|s| s.to_string());
        let t_owned: Option<String> = tag_filter.map(|t| t.to_string());
        let ts_owned: Option<String> = test_status_filter.map(|ts| ts.to_string());

        let tx = self.conn.transaction()?;
        let tasks = {
            let mut stmt = tx.prepare(&sql)?;
            let mut param_values: Vec<&dyn rusqlite::types::ToSql> = vec![&project_id];
            if let Some(ref s) = s_owned {
                param_values.push(s);
            }
            if let Some(ref t) = t_owned {
                param_values.push(t);
            }
            if let Some(ref ts) = ts_owned {
                param_values.push(ts);
            }

            let result: Result<Vec<_>, _> = stmt
                .query_map(param_values.as_slice(), row_to_task)?
                .collect();
            result?
        };
        tx.commit()?;
        Ok(tasks)
    }

    fn update_task(
        &mut self,
        id: i64,
        new_status: Option<&str>,
        new_priority: Option<&str>,
        new_tag: Option<&str>,
    ) -> Result<Task, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let old_task: Option<Task> = {
            let mut stmt = tx.prepare(
                "SELECT id, title, status, priority, tag, test_status, created_at, updated_at \
                 FROM tasks WHERE id = ?1 AND project_id = ?2",
            )?;
            let result = stmt
                .query_map(rusqlite::params![id, self.project_id], row_to_task)?
                .next()
                .transpose()?;
            result
        };
        let old_task = old_task.ok_or_else(|| format!("task #{id} not found"))?;

        let status = new_status.unwrap_or(&old_task.status);
        let priority = new_priority.unwrap_or(&old_task.priority);
        let now = time_helper::utc_now();

        if let Some(tag) = new_tag {
            tx.execute(
                "UPDATE tasks SET status = ?1, priority = ?2, updated_at = ?3, tag = ?4 \
                 WHERE id = ?5 AND project_id = ?6",
                rusqlite::params![status, priority, now, tag, id, self.project_id],
            )?;
        } else {
            tx.execute(
                "UPDATE tasks SET status = ?1, priority = ?2, updated_at = ?3 \
                 WHERE id = ?4 AND project_id = ?5",
                rusqlite::params![status, priority, now, id, self.project_id],
            )?;
        }

        tx.commit()?;
        Ok(old_task)
    }

    fn set_task_done(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let title: Option<String> = {
            let mut stmt =
                tx.prepare("SELECT title FROM tasks WHERE id = ?1 AND project_id = ?2")?;
            let result = stmt
                .query_map(rusqlite::params![id, self.project_id], |row| {
                    row.get::<_, String>(0)
                })?
                .next()
                .transpose()?;
            result
        };
        let title = title.ok_or_else(|| format!("task #{id} not found"))?;

        let now = time_helper::utc_now();
        tx.execute(
            "UPDATE tasks SET status = 'done', updated_at = ?1 WHERE id = ?2 AND project_id = ?3",
            rusqlite::params![now, id, self.project_id],
        )?;
        tx.commit()?;
        Ok(title)
    }

    fn set_task_test_status(
        &mut self,
        id: i64,
        test_status: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let title: Option<String> = {
            let mut stmt =
                tx.prepare("SELECT title FROM tasks WHERE id = ?1 AND project_id = ?2")?;
            let result = stmt
                .query_map(rusqlite::params![id, self.project_id], |row| {
                    row.get::<_, String>(0)
                })?
                .next()
                .transpose()?;
            result
        };
        let title = title.ok_or_else(|| format!("task #{id} not found"))?;

        let now = time_helper::utc_now();
        tx.execute(
            "UPDATE tasks SET test_status = ?1, updated_at = ?2 WHERE id = ?3 AND project_id = ?4",
            rusqlite::params![test_status, now, id, self.project_id],
        )?;
        tx.commit()?;
        Ok(title)
    }

    fn list_sprint_tasks(&mut self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let tasks = {
            let mut stmt = tx.prepare(
                "SELECT id, title, status, priority, tag, test_status, created_at, updated_at \
                 FROM tasks WHERE status IN ('open', 'in-progress', 'blocked') \
                 AND project_id = ?1 ORDER BY \
                 CASE priority WHEN 'critical' THEN 0 WHEN 'high' THEN 1 \
                 WHEN 'medium' THEN 2 ELSE 3 END, id",
            )?;
            let result = stmt
                .query_map(rusqlite::params![self.project_id], row_to_task)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(tasks)
    }

    fn count_tasks_by_status(&mut self) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let counts = {
            let mut stmt = tx.prepare(
                "SELECT status, COUNT(*) FROM tasks WHERE project_id = ?1 \
                 GROUP BY status ORDER BY status",
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

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
        priority: row.get(3)?,
        tag: row.get(4)?,
        test_status: row.get(5)?,
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
    use crate::store::TaskStore;
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
        let id = store
            .add_task("implement auth", "high", Some("backend"))
            .unwrap();
        assert!(id > 0);

        let task = store.get_task(id).unwrap().unwrap();
        assert_eq!(task.title, "implement auth");
        assert_eq!(task.status, "open");
        assert_eq!(task.priority, "high");
        assert_eq!(task.tag, Some("backend".to_string()));
    }

    #[test]
    fn add_without_tag() {
        let (_tmp, mut store) = setup();
        let id = store.add_task("task no tag", "medium", None).unwrap();

        let task = store.get_task(id).unwrap().unwrap();
        assert!(task.tag.is_none());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let (_tmp, mut store) = setup();
        assert!(store.get_task(999).unwrap().is_none());
    }

    #[test]
    fn list_tasks_with_filters() {
        let (_tmp, mut store) = setup();
        let id1 = store.add_task("task a", "medium", Some("api")).unwrap();
        store.add_task("task b", "high", Some("ui")).unwrap();

        let open = store.list_tasks(Some("open"), None, None).unwrap();
        assert_eq!(open.len(), 2);

        let api = store.list_tasks(None, Some("api"), None).unwrap();
        assert_eq!(api.len(), 1);
        assert_eq!(api[0].id, id1);

        let ui_open = store.list_tasks(Some("open"), Some("ui"), None).unwrap();
        assert_eq!(ui_open.len(), 1);
    }

    #[test]
    fn set_and_filter_test_status() {
        let (_tmp, mut store) = setup();
        let id = store.add_task("feature y", "medium", None).unwrap();
        // Default test_status is "planned"
        let tasks = store.list_tasks(None, None, Some("planned")).unwrap();
        assert_eq!(tasks.len(), 1);

        store.set_task_test_status(id, "implemented").unwrap();
        let tasks = store.list_tasks(None, None, Some("implemented")).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].test_status, "implemented");

        let tasks = store.list_tasks(None, None, Some("planned")).unwrap();
        assert_eq!(tasks.len(), 0);
    }

    #[test]
    fn update_task() {
        let (_tmp, mut store) = setup();
        let id = store.add_task("task", "medium", None).unwrap();

        let old = store
            .update_task(id, Some("in-progress"), Some("high"), Some("backend"))
            .unwrap();
        assert_eq!(old.status, "open");
        assert_eq!(old.priority, "medium");

        let updated = store.get_task(id).unwrap().unwrap();
        assert_eq!(updated.status, "in-progress");
        assert_eq!(updated.priority, "high");
        assert_eq!(updated.tag, Some("backend".to_string()));
    }

    #[test]
    fn update_not_found() {
        let (_tmp, mut store) = setup();
        let err = store
            .update_task(999, Some("done"), None, None)
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn set_task_done() {
        let (_tmp, mut store) = setup();
        let id = store.add_task("finish it", "medium", None).unwrap();

        let title = store.set_task_done(id).unwrap();
        assert_eq!(title, "finish it");

        let task = store.get_task(id).unwrap().unwrap();
        assert_eq!(task.status, "done");
    }

    #[test]
    fn set_done_not_found() {
        let (_tmp, mut store) = setup();
        let err = store.set_task_done(999).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn list_sprint_tasks() {
        let (_tmp, mut store) = setup();
        let id1 = store.add_task("open task", "medium", None).unwrap();
        let id2 = store.add_task("done task", "medium", None).unwrap();
        store.set_task_done(id2).unwrap();
        let id3 = store.add_task("blocked task", "high", None).unwrap();
        store.update_task(id3, Some("blocked"), None, None).unwrap();

        let sprint = store.list_sprint_tasks().unwrap();
        let ids: Vec<i64> = sprint.iter().map(|t| t.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id3));
        assert!(!ids.contains(&id2));
    }

    #[test]
    fn count_tasks_by_status() {
        let (_tmp, mut store) = setup();
        store.add_task("a", "medium", None).unwrap();
        store.add_task("b", "medium", None).unwrap();
        let id3 = store.add_task("c", "medium", None).unwrap();
        store.set_task_done(id3).unwrap();

        let counts = store.count_tasks_by_status().unwrap();
        let open_count = counts
            .iter()
            .find(|(s, _)| s == "open")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let done_count = counts
            .iter()
            .find(|(s, _)| s == "done")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        assert_eq!(open_count, 2);
        assert_eq!(done_count, 1);
    }

    #[test]
    fn priority_ordering() {
        let (_tmp, mut store) = setup();
        store.add_task("low task", "low", None).unwrap();
        store.add_task("critical task", "critical", None).unwrap();
        store.add_task("high task", "high", None).unwrap();

        let tasks = store.list_tasks(None, None, None).unwrap();
        assert_eq!(tasks[0].priority, "critical");
        assert_eq!(tasks[1].priority, "high");
        assert_eq!(tasks[2].priority, "low");
    }

    #[test]
    fn auto_increment_ids() {
        let (_tmp, mut store) = setup();
        let id1 = store.add_task("first", "medium", None).unwrap();
        let id2 = store.add_task("second", "medium", None).unwrap();
        assert!(id2 > id1);
    }
}
