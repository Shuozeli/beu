use super::SqliteStore;
use crate::time_helper;

pub(super) fn create_tables(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}

impl SqliteStore {
    /// Register the current project in the projects table. Idempotent.
    pub fn register_project(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO projects (id, created_at) VALUES (?1, ?2)",
            rusqlite::params![self.project_id, now],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// List all registered projects.
    #[cfg(test)]
    pub fn list_projects(&mut self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let projects = {
            let mut stmt = tx.prepare("SELECT id, created_at FROM projects ORDER BY id")?;
            let result = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(projects)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup(project_id: &str) -> (TempDir, SqliteStore) {
        let tmp = TempDir::new().unwrap();
        let beu_dir = tmp.path().join(".beu");
        let store = SqliteStore::open(&beu_dir, project_id).unwrap();
        (tmp, store)
    }

    #[test]
    fn register_project_creates_entry() {
        let (_tmp, mut store) = setup("default");
        store.register_project().unwrap();
        let projects = store.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].0, "default");
    }

    #[test]
    fn register_project_is_idempotent() {
        let (_tmp, mut store) = setup("default");
        store.register_project().unwrap();
        store.register_project().unwrap();
        let projects = store.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
    }

    #[test]
    fn list_projects_sorted() {
        let (tmp, mut store) = setup("alpha");
        store.register_project().unwrap();
        drop(store);

        let beu_dir = tmp.path().join(".beu");
        let mut store2 = SqliteStore::open(&beu_dir, "beta").unwrap();
        store2.register_project().unwrap();

        let projects = store2.list_projects().unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].0, "alpha");
        assert_eq!(projects[1].0, "beta");
    }
}
