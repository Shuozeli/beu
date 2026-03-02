use crate::store::StateStore;

const VALID_CATEGORIES: &[&str] = &["decision", "blocker", "focus", "note"];

pub fn cmd_set(
    store: &mut impl StateStore,
    category: &str,
    key: &str,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_category(category)?;
    store.set(category, key, value)?;
    println!("[{category}] {key}: {value}");
    Ok(())
}

pub fn cmd_get(
    store: &mut impl StateStore,
    key: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        Some(k) => {
            let entry = store
                .get_by_key(k)?
                .ok_or_else(|| format!("state entry '{k}' not found"))?;

            println!("Key: {k}");
            println!("Category: {}", entry.category);
            println!("Value: {}", entry.value);
            println!("Created: {}", entry.created_at);
            println!("Updated: {}", entry.updated_at);
        }
        None => {
            let entries = store.list_entries(None)?;

            if entries.is_empty() {
                println!(
                    "No state entries. Use 'beu state set --category <C> <key> <value>' to add."
                );
                return Ok(());
            }

            for entry in &entries {
                println!(
                    "  [{category}] {key}: {value}",
                    category = entry.category,
                    key = entry.key,
                    value = entry.value
                );
            }
        }
    }

    Ok(())
}

pub fn cmd_list(
    store: &mut impl StateStore,
    category: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(cat) = category {
        validate_category(cat)?;
    }

    let entries = store.list_entries(category)?;

    if entries.is_empty() {
        if let Some(cat) = category {
            println!("No state entries with category '{cat}'.");
        } else {
            println!("No state entries. Use 'beu state set --category <C> <key> <value>' to add.");
        }
        return Ok(());
    }

    for entry in &entries {
        println!(
            "  [{category}] {key}: {value}",
            category = entry.category,
            key = entry.key,
            value = entry.value
        );
    }

    Ok(())
}

pub fn cmd_remove(
    store: &mut impl StateStore,
    key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !store.remove(key)? {
        return Err(format!("state entry '{key}' not found").into());
    }

    println!("Removed state entry '{key}'.");
    Ok(())
}

pub fn cmd_clear(
    store: &mut impl StateStore,
    category: &str,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_category(category)?;

    if !force {
        return Err(
            format!("this will delete all '{category}' entries. Use --force to confirm.").into(),
        );
    }

    let count = store.clear_category(category)?;
    println!("Cleared {count} '{category}' entry(ies).");
    Ok(())
}

fn validate_category(category: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !VALID_CATEGORIES.contains(&category) {
        return Err(format!(
            "invalid category '{category}' (valid: {})",
            VALID_CATEGORIES.join(", ")
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::StateEntry;
    use std::collections::HashMap;

    struct FakeStateStore {
        entries: HashMap<(String, String), StateEntry>,
    }

    impl FakeStateStore {
        fn new() -> Self {
            Self {
                entries: HashMap::new(),
            }
        }
    }

    impl StateStore for FakeStateStore {
        fn set(
            &mut self,
            category: &str,
            key: &str,
            value: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let k = (category.to_string(), key.to_string());
            self.entries.insert(
                k,
                StateEntry {
                    category: category.to_string(),
                    key: key.to_string(),
                    value: value.to_string(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    updated_at: "2024-01-01T00:00:00Z".to_string(),
                },
            );
            Ok(())
        }

        fn get_by_key(
            &mut self,
            key: &str,
        ) -> Result<Option<StateEntry>, Box<dyn std::error::Error>> {
            Ok(self.entries.values().find(|e| e.key == key).cloned())
        }

        fn list_entries(
            &mut self,
            category: Option<&str>,
        ) -> Result<Vec<StateEntry>, Box<dyn std::error::Error>> {
            let mut result: Vec<_> = self
                .entries
                .values()
                .filter(|e| category.map_or(true, |c| e.category == c))
                .cloned()
                .collect();
            result.sort_by(|a, b| (&a.category, &a.key).cmp(&(&b.category, &b.key)));
            Ok(result)
        }

        fn remove(&mut self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
            let to_remove: Vec<_> = self
                .entries
                .keys()
                .filter(|(_, k)| k == key)
                .cloned()
                .collect();
            let found = !to_remove.is_empty();
            for k in to_remove {
                self.entries.remove(&k);
            }
            Ok(found)
        }

        fn clear_category(&mut self, category: &str) -> Result<u64, Box<dyn std::error::Error>> {
            let to_remove: Vec<_> = self
                .entries
                .keys()
                .filter(|(c, _)| c == category)
                .cloned()
                .collect();
            let count = to_remove.len() as u64;
            for k in to_remove {
                self.entries.remove(&k);
            }
            Ok(count)
        }

        fn count_by_category(&mut self, category: &str) -> Result<i64, Box<dyn std::error::Error>> {
            Ok(self
                .entries
                .values()
                .filter(|e| e.category == category)
                .count() as i64)
        }

        fn get_checkpoint(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
            Ok(self
                .entries
                .get(&("focus".to_string(), "_checkpoint".to_string()))
                .map(|e| e.value.clone()))
        }

        fn clear_checkpoint(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.entries
                .remove(&("focus".to_string(), "_checkpoint".to_string()));
            Ok(())
        }

        fn list_blockers(&mut self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
            let mut result: Vec<_> = self
                .entries
                .values()
                .filter(|e| e.category == "blocker")
                .map(|e| (e.key.clone(), e.value.clone()))
                .collect();
            result.sort();
            Ok(result)
        }

        fn list_focus_items(
            &mut self,
        ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
            let mut result: Vec<_> = self
                .entries
                .values()
                .filter(|e| e.category == "focus" && e.key != "_checkpoint")
                .map(|e| (e.key.clone(), e.value.clone()))
                .collect();
            result.sort();
            Ok(result)
        }
    }

    #[test]
    fn set_and_get() {
        let mut store = FakeStateStore::new();
        cmd_set(&mut store, "decision", "db-engine", "postgres").unwrap();
        let entry = store.get_by_key("db-engine").unwrap().unwrap();
        assert_eq!(entry.value, "postgres");
        assert_eq!(entry.category, "decision");
    }

    #[test]
    fn upsert_overwrites() {
        let mut store = FakeStateStore::new();
        cmd_set(&mut store, "decision", "db", "mysql").unwrap();
        cmd_set(&mut store, "decision", "db", "postgres").unwrap();
        let entry = store.get_by_key("db").unwrap().unwrap();
        assert_eq!(entry.value, "postgres");
    }

    #[test]
    fn invalid_category_fails() {
        let mut store = FakeStateStore::new();
        let err = cmd_set(&mut store, "invalid", "key", "val").unwrap_err();
        assert!(err.to_string().contains("invalid category"));
    }

    #[test]
    fn remove_entry() {
        let mut store = FakeStateStore::new();
        cmd_set(&mut store, "note", "key1", "val1").unwrap();
        cmd_remove(&mut store, "key1").unwrap();
        assert!(store.entries.is_empty());
    }

    #[test]
    fn remove_not_found() {
        let mut store = FakeStateStore::new();
        let err = cmd_remove(&mut store, "nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn clear_with_force() {
        let mut store = FakeStateStore::new();
        cmd_set(&mut store, "blocker", "b1", "issue1").unwrap();
        cmd_set(&mut store, "blocker", "b2", "issue2").unwrap();
        cmd_set(&mut store, "note", "n1", "note1").unwrap();

        cmd_clear(&mut store, "blocker", true).unwrap();

        assert_eq!(store.entries.len(), 1);
        assert!(store.entries.values().next().unwrap().category == "note");
    }

    #[test]
    fn clear_without_force_fails() {
        let mut store = FakeStateStore::new();
        let err = cmd_clear(&mut store, "blocker", false).unwrap_err();
        assert!(err.to_string().contains("--force"));
    }
}
