use crate::store::IdeaStore;

const VALID_AREAS: &[&str] = &[
    "api", "ui", "database", "testing", "docs", "tooling", "general",
];
const VALID_STATUSES: &[&str] = &["pending", "done", "archived"];
const VALID_PRIORITIES: &[&str] = &["low", "medium", "high"];

pub fn cmd_add(
    store: &mut impl IdeaStore,
    title: &str,
    area: &str,
    priority: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !VALID_AREAS.contains(&area) {
        return Err(format!("invalid area '{area}' (valid: {})", VALID_AREAS.join(", ")).into());
    }
    if !VALID_PRIORITIES.contains(&priority) {
        return Err(format!(
            "invalid priority '{priority}' (valid: {})",
            VALID_PRIORITIES.join(", ")
        )
        .into());
    }

    let id = store.add_idea(title, area, priority)?;
    println!("#{id}: {title} [{area}] ({priority})");
    Ok(())
}

pub fn cmd_list(
    store: &mut impl IdeaStore,
    area: Option<&str>,
    status: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(a) = area {
        if !VALID_AREAS.contains(&a) {
            return Err(format!("invalid area '{a}' (valid: {})", VALID_AREAS.join(", ")).into());
        }
    }
    if let Some(s) = status {
        if !VALID_STATUSES.contains(&s) {
            return Err(format!(
                "invalid status '{s}' (valid: {})",
                VALID_STATUSES.join(", ")
            )
            .into());
        }
    }

    let ideas = store.list_ideas(area, status)?;

    if ideas.is_empty() {
        println!("No ideas found. Use 'beu idea add <title>' to capture ideas.");
        return Ok(());
    }

    for idea in &ideas {
        let priority_marker = match idea.priority.as_str() {
            "high" => "!! ",
            "medium" => "!  ",
            _ => "   ",
        };
        println!(
            "{priority_marker}#{id} [{status}] [{area}] {title}",
            id = idea.id,
            status = idea.status,
            area = idea.area,
            title = idea.title
        );
    }

    Ok(())
}

pub fn cmd_show(store: &mut impl IdeaStore, id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let idea = store
        .get_idea(id)?
        .ok_or_else(|| format!("idea #{id} not found"))?;

    println!("Idea #{}: {}", idea.id, idea.title);
    println!("Area: {}", idea.area);
    println!("Status: {}", idea.status);
    println!("Priority: {}", idea.priority);
    if let Some(ref desc) = idea.description {
        println!("Description: {desc}");
    }
    println!("Created: {}", idea.created_at);
    println!("Updated: {}", idea.updated_at);
    Ok(())
}

pub fn cmd_done(store: &mut impl IdeaStore, id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let title = store.set_idea_done(id)?;
    println!("#{id} done: {title}");
    Ok(())
}

pub fn cmd_archive(store: &mut impl IdeaStore, id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let title = store.archive_idea(id)?;
    println!("#{id} archived: {title}");
    Ok(())
}

pub fn cmd_describe(
    store: &mut impl IdeaStore,
    id: i64,
    description: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    store.describe_idea(id, description)?;
    println!("#{id}: description updated");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Idea;

    struct FakeIdeaStore {
        ideas: Vec<Idea>,
        next_id: i64,
    }

    impl FakeIdeaStore {
        fn new() -> Self {
            Self {
                ideas: Vec::new(),
                next_id: 1,
            }
        }
    }

    impl IdeaStore for FakeIdeaStore {
        fn add_idea(
            &mut self,
            title: &str,
            area: &str,
            priority: &str,
        ) -> Result<i64, Box<dyn std::error::Error>> {
            let id = self.next_id;
            self.next_id += 1;
            self.ideas.push(Idea {
                id,
                title: title.to_string(),
                area: area.to_string(),
                description: None,
                status: "pending".to_string(),
                priority: priority.to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            });
            Ok(id)
        }

        fn get_idea(&mut self, id: i64) -> Result<Option<Idea>, Box<dyn std::error::Error>> {
            Ok(self.ideas.iter().find(|i| i.id == id).cloned())
        }

        fn list_ideas(
            &mut self,
            area: Option<&str>,
            status: Option<&str>,
        ) -> Result<Vec<Idea>, Box<dyn std::error::Error>> {
            Ok(self
                .ideas
                .iter()
                .filter(|i| area.map_or(true, |a| i.area == a))
                .filter(|i| status.map_or(true, |s| i.status == s))
                .cloned()
                .collect())
        }

        fn set_idea_done(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>> {
            let idea = self
                .ideas
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| format!("idea #{id} not found"))?;
            idea.status = "done".to_string();
            Ok(idea.title.clone())
        }

        fn archive_idea(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>> {
            let idea = self
                .ideas
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| format!("idea #{id} not found"))?;
            idea.status = "archived".to_string();
            Ok(idea.title.clone())
        }

        fn describe_idea(
            &mut self,
            id: i64,
            description: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let idea = self
                .ideas
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| format!("idea #{id} not found"))?;
            idea.description = Some(description.to_string());
            Ok(())
        }

        fn count_ideas_by_status(
            &mut self,
        ) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error>> {
            let mut counts = std::collections::HashMap::new();
            for i in &self.ideas {
                *counts.entry(i.status.clone()).or_insert(0i64) += 1;
            }
            let mut result: Vec<_> = counts.into_iter().collect();
            result.sort();
            Ok(result)
        }
    }

    #[test]
    fn add_and_query() {
        let mut store = FakeIdeaStore::new();
        cmd_add(&mut store, "rate limiting", "api", "high").unwrap();
        assert_eq!(store.ideas.len(), 1);
        assert_eq!(store.ideas[0].title, "rate limiting");
        assert_eq!(store.ideas[0].area, "api");
    }

    #[test]
    fn invalid_area_fails() {
        let mut store = FakeIdeaStore::new();
        let err = cmd_add(&mut store, "task", "invalid", "medium").unwrap_err();
        assert!(err.to_string().contains("invalid area"));
    }

    #[test]
    fn done_and_archive() {
        let mut store = FakeIdeaStore::new();
        cmd_add(&mut store, "idea one", "general", "medium").unwrap();
        cmd_add(&mut store, "idea two", "general", "medium").unwrap();

        cmd_done(&mut store, 1).unwrap();
        cmd_archive(&mut store, 2).unwrap();

        assert_eq!(store.ideas[0].status, "done");
        assert_eq!(store.ideas[1].status, "archived");
    }

    #[test]
    fn describe_updates_description() {
        let mut store = FakeIdeaStore::new();
        cmd_add(&mut store, "design api", "api", "medium").unwrap();
        cmd_describe(&mut store, 1, "Detailed design document").unwrap();
        assert_eq!(
            store.ideas[0].description,
            Some("Detailed design document".to_string())
        );
    }

    #[test]
    fn show_not_found() {
        let mut store = FakeIdeaStore::new();
        let err = cmd_show(&mut store, 999).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
