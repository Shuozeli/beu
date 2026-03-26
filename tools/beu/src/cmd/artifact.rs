use crate::store::ArtifactStore;

const VALID_TYPES: &[&str] = &["doc", "codelab", "test", "config", "spec", "changelog"];
const VALID_STATUSES: &[&str] = &["pending", "in-progress", "review", "done"];

pub fn cmd_add(
    store: &mut impl ArtifactStore,
    name: &str,
    artifact_type: &str,
    description: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !VALID_TYPES.contains(&artifact_type) {
        return Err(format!(
            "invalid artifact type '{artifact_type}' (valid: {})",
            VALID_TYPES.join(", ")
        )
        .into());
    }
    if store.get_artifact(name)?.is_some() {
        return Err(format!("artifact '{name}' already exists").into());
    }

    store.add_artifact(name, artifact_type, description)?;
    println!("Tracking '{name}' ({artifact_type}) - status: pending");
    Ok(())
}

pub fn cmd_status(
    store: &mut impl ArtifactStore,
    name: &str,
    new_status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !VALID_STATUSES.contains(&new_status) {
        return Err(format!(
            "invalid status '{new_status}' (valid: {})",
            VALID_STATUSES.join(", ")
        )
        .into());
    }

    let old_status = store.update_artifact_status(name, new_status)?;
    println!("'{name}': {old_status} -> {new_status}");
    Ok(())
}

pub fn cmd_list(
    store: &mut impl ArtifactStore,
    filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = store.list_artifacts(filter)?;

    if artifacts.is_empty() {
        if let Some(s) = filter {
            println!("No artifacts with status '{s}'.");
        } else {
            println!("No artifacts tracked yet. Use 'beu artifact add <name>' to start.");
        }
        return Ok(());
    }

    for a in &artifacts {
        println!(
            "  [{status}] {name} ({atype}) - updated {updated}",
            status = a.status,
            name = a.name,
            atype = a.artifact_type,
            updated = a.updated_at
        );
    }

    Ok(())
}

pub fn cmd_show(
    store: &mut impl ArtifactStore,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifact = store
        .get_artifact(name)?
        .ok_or_else(|| format!("artifact '{name}' not found"))?;

    println!("Artifact: {}", artifact.name);
    println!("Type: {}", artifact.artifact_type);
    println!("Status: {}", artifact.status);
    if let Some(ref desc) = artifact.description {
        println!("Description: {desc}");
    }
    println!("Created: {}", artifact.created_at);
    println!("Updated: {}", artifact.updated_at);
    Ok(())
}

pub fn cmd_describe(
    store: &mut impl ArtifactStore,
    name: &str,
    description: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    store.describe_artifact(name, description)?;
    println!("'{name}': description updated");
    Ok(())
}

pub fn cmd_remove(
    store: &mut impl ArtifactStore,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !store.remove_artifact(name)? {
        return Err(format!("artifact '{name}' not found").into());
    }

    println!("Removed artifact '{name}'.");
    Ok(())
}

pub fn cmd_changelog(
    store: &mut impl ArtifactStore,
    name: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    store.add_changelog_entry(name, message)?;
    println!("'{name}': changelog entry added");
    Ok(())
}

pub fn cmd_history(
    store: &mut impl ArtifactStore,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = store
        .get_artifact(name)?
        .ok_or_else(|| format!("artifact '{name}' not found"))?;

    let entries = store.list_changelog(name)?;

    if entries.is_empty() {
        println!("No changelog entries for '{name}'.");
        return Ok(());
    }

    println!("Changelog for '{name}':");
    for entry in &entries {
        println!("  {}: {}", entry.created_at, entry.message);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Artifact, ArtifactChangelog};
    use std::collections::HashMap;

    struct FakeArtifactStore {
        artifacts: HashMap<String, Artifact>,
        changelog: HashMap<String, Vec<ArtifactChangelog>>,
    }

    impl FakeArtifactStore {
        fn new() -> Self {
            Self {
                artifacts: HashMap::new(),
                changelog: HashMap::new(),
            }
        }
    }

    impl ArtifactStore for FakeArtifactStore {
        fn add_artifact(
            &mut self,
            name: &str,
            artifact_type: &str,
            description: Option<&str>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.artifacts.insert(
                name.to_string(),
                Artifact {
                    name: name.to_string(),
                    artifact_type: artifact_type.to_string(),
                    description: description.map(|d| d.to_string()),
                    status: "pending".to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    updated_at: "2024-01-01T00:00:00Z".to_string(),
                },
            );
            Ok(())
        }

        fn get_artifact(
            &mut self,
            name: &str,
        ) -> Result<Option<Artifact>, Box<dyn std::error::Error>> {
            Ok(self.artifacts.get(name).cloned())
        }

        fn update_artifact_status(
            &mut self,
            name: &str,
            new_status: &str,
        ) -> Result<String, Box<dyn std::error::Error>> {
            let a = self
                .artifacts
                .get_mut(name)
                .ok_or_else(|| format!("artifact '{name}' not found"))?;
            let old = a.status.clone();
            a.status = new_status.to_string();
            Ok(old)
        }

        fn describe_artifact(
            &mut self,
            name: &str,
            description: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let a = self
                .artifacts
                .get_mut(name)
                .ok_or_else(|| format!("artifact '{name}' not found"))?;
            a.description = Some(description.to_string());
            Ok(())
        }

        fn list_artifacts(
            &mut self,
            status_filter: Option<&str>,
        ) -> Result<Vec<Artifact>, Box<dyn std::error::Error>> {
            let mut result: Vec<_> = self
                .artifacts
                .values()
                .filter(|a| status_filter.map_or(true, |s| a.status == s))
                .cloned()
                .collect();
            result.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(result)
        }

        fn remove_artifact(&mut self, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
            if self.artifacts.remove(name).is_some() {
                self.changelog.remove(name);
                Ok(true)
            } else {
                Ok(false)
            }
        }

        fn add_changelog_entry(
            &mut self,
            name: &str,
            message: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            if !self.artifacts.contains_key(name) {
                return Err(format!("artifact '{name}' not found").into());
            }
            self.changelog
                .entry(name.to_string())
                .or_default()
                .push(ArtifactChangelog {
                    message: message.to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                });
            Ok(())
        }

        fn list_changelog(
            &mut self,
            name: &str,
        ) -> Result<Vec<ArtifactChangelog>, Box<dyn std::error::Error>> {
            Ok(self.changelog.get(name).cloned().unwrap_or_default())
        }
    }

    #[test]
    fn add_and_show() {
        let mut store = FakeArtifactStore::new();
        cmd_add(&mut store, "design-doc", "doc", None).unwrap();
        assert!(store.artifacts.contains_key("design-doc"));
        assert_eq!(store.artifacts["design-doc"].description, None);
    }

    #[test]
    fn add_with_description() {
        let mut store = FakeArtifactStore::new();
        cmd_add(&mut store, "api-spec", "spec", Some("OpenAPI spec for v2")).unwrap();
        assert_eq!(
            store.artifacts["api-spec"].description,
            Some("OpenAPI spec for v2".to_string())
        );
    }

    #[test]
    fn duplicate_fails() {
        let mut store = FakeArtifactStore::new();
        cmd_add(&mut store, "doc1", "doc", None).unwrap();
        let err = cmd_add(&mut store, "doc1", "doc", None).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn invalid_status_fails() {
        let mut store = FakeArtifactStore::new();
        cmd_add(&mut store, "thing", "doc", None).unwrap();
        let err = cmd_status(&mut store, "thing", "invalid").unwrap_err();
        assert!(err.to_string().contains("invalid status"));
    }

    #[test]
    fn status_update() {
        let mut store = FakeArtifactStore::new();
        cmd_add(&mut store, "spec", "spec", None).unwrap();
        cmd_status(&mut store, "spec", "in-progress").unwrap();
        assert_eq!(store.artifacts["spec"].status, "in-progress");
    }

    #[test]
    fn describe_updates_description() {
        let mut store = FakeArtifactStore::new();
        cmd_add(&mut store, "readme", "doc", None).unwrap();
        cmd_describe(&mut store, "readme", "Project README").unwrap();
        assert_eq!(
            store.artifacts["readme"].description,
            Some("Project README".to_string())
        );
    }

    #[test]
    fn describe_not_found() {
        let mut store = FakeArtifactStore::new();
        let err = cmd_describe(&mut store, "nonexistent", "desc").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn remove_artifact() {
        let mut store = FakeArtifactStore::new();
        cmd_add(&mut store, "old", "doc", None).unwrap();
        cmd_remove(&mut store, "old").unwrap();
        assert!(store.artifacts.is_empty());
    }

    #[test]
    fn remove_not_found() {
        let mut store = FakeArtifactStore::new();
        let err = cmd_remove(&mut store, "nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn show_not_found() {
        let mut store = FakeArtifactStore::new();
        let err = cmd_show(&mut store, "nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn changelog_adds_entry() {
        let mut store = FakeArtifactStore::new();
        cmd_add(&mut store, "readme", "doc", None).unwrap();
        cmd_changelog(&mut store, "readme", "initial draft").unwrap();

        let entries = &store.changelog["readme"];
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "initial draft");
    }

    #[test]
    fn changelog_not_found() {
        let mut store = FakeArtifactStore::new();
        let err = cmd_changelog(&mut store, "nope", "msg").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn history_not_found() {
        let mut store = FakeArtifactStore::new();
        let err = cmd_history(&mut store, "nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
