use crate::store::{Artifact, ArtifactChangelog, ArtifactStore};
use crate::time_helper;

use super::SqliteStore;

pub(super) fn create_tables(tx: &rusqlite::Transaction) -> Result<(), Box<dyn std::error::Error>> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS artifacts (
            name TEXT NOT NULL,
            artifact_type TEXT NOT NULL DEFAULT 'doc',
            description TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            project_id TEXT NOT NULL DEFAULT 'default',
            PRIMARY KEY (project_id, name)
        );
        CREATE TABLE IF NOT EXISTS artifact_changelog (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            artifact_name TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at TEXT NOT NULL,
            project_id TEXT NOT NULL DEFAULT 'default',
            FOREIGN KEY (project_id, artifact_name) REFERENCES artifacts(project_id, name)
        );",
    )?;
    super::migrate_add_project_id(tx, "artifacts")?;
    super::migrate_add_project_id(tx, "artifact_changelog")?;
    Ok(())
}

impl ArtifactStore for SqliteStore {
    fn add_artifact(
        &mut self,
        name: &str,
        artifact_type: &str,
        description: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = time_helper::utc_now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO artifacts (name, artifact_type, description, status, created_at, updated_at, project_id) \
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6)",
            rusqlite::params![name, artifact_type, description, now, now, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn get_artifact(&mut self, name: &str) -> Result<Option<Artifact>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let result = {
            let mut stmt = tx.prepare(
                "SELECT name, artifact_type, description, status, created_at, updated_at \
                 FROM artifacts WHERE name = ?1 AND project_id = ?2",
            )?;
            let result = stmt
                .query_map(rusqlite::params![name, self.project_id], row_to_artifact)?
                .next()
                .transpose()?;
            result
        };
        tx.commit()?;
        Ok(result)
    }

    fn update_artifact_status(
        &mut self,
        name: &str,
        new_status: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let old_status: Option<String> = {
            let mut stmt =
                tx.prepare("SELECT status FROM artifacts WHERE name = ?1 AND project_id = ?2")?;
            let result = stmt
                .query_map(rusqlite::params![name, self.project_id], |row| {
                    row.get::<_, String>(0)
                })?
                .next()
                .transpose()?;
            result
        };
        let old_status = old_status.ok_or_else(|| format!("artifact '{name}' not found"))?;

        let now = time_helper::utc_now();
        tx.execute(
            "UPDATE artifacts SET status = ?1, updated_at = ?2 WHERE name = ?3 AND project_id = ?4",
            rusqlite::params![new_status, now, name, self.project_id],
        )?;
        tx.commit()?;
        Ok(old_status)
    }

    fn describe_artifact(
        &mut self,
        name: &str,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let exists = tx
            .prepare("SELECT 1 FROM artifacts WHERE name = ?1 AND project_id = ?2")?
            .exists(rusqlite::params![name, self.project_id])?;
        if !exists {
            return Err(format!("artifact '{name}' not found").into());
        }

        let now = time_helper::utc_now();
        tx.execute(
            "UPDATE artifacts SET description = ?1, updated_at = ?2 WHERE name = ?3 AND project_id = ?4",
            rusqlite::params![description, now, name, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn list_artifacts(
        &mut self,
        status_filter: Option<&str>,
    ) -> Result<Vec<Artifact>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let artifacts = if let Some(status) = status_filter {
            let mut stmt = tx.prepare(
                "SELECT name, artifact_type, description, status, created_at, updated_at \
                 FROM artifacts WHERE status = ?1 AND project_id = ?2 ORDER BY name",
            )?;
            let result = stmt
                .query_map(rusqlite::params![status, self.project_id], row_to_artifact)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        } else {
            let mut stmt = tx.prepare(
                "SELECT name, artifact_type, description, status, created_at, updated_at \
                 FROM artifacts WHERE project_id = ?1 ORDER BY name",
            )?;
            let result = stmt
                .query_map(rusqlite::params![self.project_id], row_to_artifact)?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };

        tx.commit()?;
        Ok(artifacts)
    }

    fn remove_artifact(&mut self, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let exists = tx
            .prepare("SELECT 1 FROM artifacts WHERE name = ?1 AND project_id = ?2")?
            .exists(rusqlite::params![name, self.project_id])?;
        if !exists {
            tx.commit()?;
            return Ok(false);
        }

        tx.execute(
            "DELETE FROM artifact_changelog WHERE artifact_name = ?1 AND project_id = ?2",
            rusqlite::params![name, self.project_id],
        )?;
        tx.execute(
            "DELETE FROM artifacts WHERE name = ?1 AND project_id = ?2",
            rusqlite::params![name, self.project_id],
        )?;
        tx.commit()?;
        Ok(true)
    }

    fn add_changelog_entry(
        &mut self,
        name: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let exists = tx
            .prepare("SELECT 1 FROM artifacts WHERE name = ?1 AND project_id = ?2")?
            .exists(rusqlite::params![name, self.project_id])?;
        if !exists {
            return Err(format!("artifact '{name}' not found").into());
        }

        let now = time_helper::utc_now();
        tx.execute(
            "INSERT INTO artifact_changelog (artifact_name, message, created_at, project_id) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, message, now, self.project_id],
        )?;
        tx.execute(
            "UPDATE artifacts SET updated_at = ?1 WHERE name = ?2 AND project_id = ?3",
            rusqlite::params![now, name, self.project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn list_changelog(
        &mut self,
        name: &str,
    ) -> Result<Vec<ArtifactChangelog>, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let entries = {
            let mut stmt = tx.prepare(
                "SELECT message, created_at FROM artifact_changelog \
                 WHERE artifact_name = ?1 AND project_id = ?2 ORDER BY id ASC",
            )?;
            let result = stmt
                .query_map(rusqlite::params![name, self.project_id], |row| {
                    Ok(ArtifactChangelog {
                        message: row.get(0)?,
                        created_at: row.get(1)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            result
        };
        tx.commit()?;
        Ok(entries)
    }
}

fn row_to_artifact(row: &rusqlite::Row<'_>) -> rusqlite::Result<Artifact> {
    Ok(Artifact {
        name: row.get(0)?,
        artifact_type: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::SqliteStore;
    use crate::store::ArtifactStore;
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
        store.add_artifact("readme", "doc", None).unwrap();

        let a = store.get_artifact("readme").unwrap().unwrap();
        assert_eq!(a.name, "readme");
        assert_eq!(a.artifact_type, "doc");
        assert_eq!(a.status, "pending");
        assert!(a.description.is_none());
    }

    #[test]
    fn add_with_description() {
        let (_tmp, mut store) = setup();
        store
            .add_artifact("api-spec", "spec", Some("OpenAPI v3 spec"))
            .unwrap();

        let a = store.get_artifact("api-spec").unwrap().unwrap();
        assert_eq!(a.description, Some("OpenAPI v3 spec".to_string()));
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let (_tmp, mut store) = setup();
        assert!(store.get_artifact("nope").unwrap().is_none());
    }

    #[test]
    fn update_status() {
        let (_tmp, mut store) = setup();
        store.add_artifact("doc", "doc", None).unwrap();

        let old = store.update_artifact_status("doc", "in-progress").unwrap();
        assert_eq!(old, "pending");

        let a = store.get_artifact("doc").unwrap().unwrap();
        assert_eq!(a.status, "in-progress");
    }

    #[test]
    fn update_status_not_found() {
        let (_tmp, mut store) = setup();
        let err = store.update_artifact_status("nope", "done").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn describe_artifact() {
        let (_tmp, mut store) = setup();
        store.add_artifact("readme", "doc", None).unwrap();
        store.describe_artifact("readme", "Project README").unwrap();

        let a = store.get_artifact("readme").unwrap().unwrap();
        assert_eq!(a.description, Some("Project README".to_string()));
    }

    #[test]
    fn describe_not_found() {
        let (_tmp, mut store) = setup();
        let err = store.describe_artifact("nope", "desc").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn list_artifacts_all() {
        let (_tmp, mut store) = setup();
        store.add_artifact("b-doc", "doc", None).unwrap();
        store.add_artifact("a-spec", "spec", None).unwrap();

        let all = store.list_artifacts(None).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "a-spec");
        assert_eq!(all[1].name, "b-doc");
    }

    #[test]
    fn list_artifacts_with_filter() {
        let (_tmp, mut store) = setup();
        store.add_artifact("one", "doc", None).unwrap();
        store.add_artifact("two", "doc", None).unwrap();
        store.update_artifact_status("two", "done").unwrap();

        let pending = store.list_artifacts(Some("pending")).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].name, "one");

        let done = store.list_artifacts(Some("done")).unwrap();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].name, "two");
    }

    #[test]
    fn remove_artifact() {
        let (_tmp, mut store) = setup();
        store.add_artifact("temp", "doc", None).unwrap();

        assert!(store.remove_artifact("temp").unwrap());
        assert!(store.get_artifact("temp").unwrap().is_none());
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let (_tmp, mut store) = setup();
        assert!(!store.remove_artifact("nope").unwrap());
    }

    #[test]
    fn duplicate_add_fails() {
        let (_tmp, mut store) = setup();
        store.add_artifact("dup", "doc", None).unwrap();
        let err = store.add_artifact("dup", "doc", None).unwrap_err();
        assert!(err.to_string().contains("UNIQUE") || err.to_string().contains("constraint"));
    }

    #[test]
    fn add_and_list_changelog() {
        let (_tmp, mut store) = setup();
        store.add_artifact("readme", "doc", None).unwrap();
        store
            .add_changelog_entry("readme", "initial draft")
            .unwrap();
        store
            .add_changelog_entry("readme", "added installation section")
            .unwrap();

        let entries = store.list_changelog("readme").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "initial draft");
        assert_eq!(entries[1].message, "added installation section");
    }

    #[test]
    fn changelog_not_found() {
        let (_tmp, mut store) = setup();
        let err = store.add_changelog_entry("nope", "msg").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn changelog_empty_for_new_artifact() {
        let (_tmp, mut store) = setup();
        store.add_artifact("spec", "spec", None).unwrap();
        let entries = store.list_changelog("spec").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn changelog_isolated_by_artifact() {
        let (_tmp, mut store) = setup();
        store.add_artifact("a", "doc", None).unwrap();
        store.add_artifact("b", "doc", None).unwrap();
        store.add_changelog_entry("a", "change to a").unwrap();
        store.add_changelog_entry("b", "change to b").unwrap();

        assert_eq!(store.list_changelog("a").unwrap().len(), 1);
        assert_eq!(store.list_changelog("b").unwrap().len(), 1);
    }

    #[test]
    fn remove_artifact_cascades_changelog() {
        let (_tmp, mut store) = setup();
        store.add_artifact("temp", "doc", None).unwrap();
        store.add_changelog_entry("temp", "some change").unwrap();

        assert!(store.remove_artifact("temp").unwrap());
        let entries = store.list_changelog("temp").unwrap();
        assert!(entries.is_empty());
    }
}
