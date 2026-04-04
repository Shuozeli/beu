use crate::store::TaskStore;

pub fn cmd_add(
    store: &mut impl TaskStore,
    title: &str,
    priority: &str,
    tag: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let valid_priority = ["low", "medium", "high", "critical"];
    if !valid_priority.contains(&priority) {
        return Err(format!(
            "invalid priority '{priority}' (valid: {})",
            valid_priority.join(", ")
        )
        .into());
    }

    let id = store.add_task(title, priority, tag)?;
    let tag_info = tag.map(|t| format!(" [{t}]")).unwrap_or_default();
    println!("#{id}: {title}{tag_info} ({priority})");
    Ok(())
}

pub fn cmd_list(
    store: &mut impl TaskStore,
    status_filter: Option<&str>,
    tag_filter: Option<&str>,
    test_status_filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let tasks = store.list_tasks(status_filter, tag_filter, test_status_filter)?;

    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    for t in &tasks {
        let tag_str = t
            .tag
            .as_ref()
            .map(|t| format!(" [{t}]"))
            .unwrap_or_default();
        let pri_marker = match t.priority.as_str() {
            "critical" => "!!!",
            "high" => "!! ",
            "medium" => "!  ",
            _ => "   ",
        };
        println!(
            "  {pri_marker} #{id} [{status}]{tag_str} {title}  (test:{ts})",
            id = t.id,
            status = t.status,
            title = t.title,
            ts = t.test_status,
        );
    }

    Ok(())
}

pub fn cmd_update(
    store: &mut impl TaskStore,
    id: i64,
    new_status: Option<&str>,
    new_priority: Option<&str>,
    new_tag: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if new_status.is_none() && new_priority.is_none() && new_tag.is_none() {
        return Err("nothing to update. Use --status, --priority, or --tag.".into());
    }

    if let Some(s) = new_status {
        let valid = ["open", "in-progress", "done", "blocked"];
        if !valid.contains(&s) {
            return Err(format!("invalid status '{s}' (valid: {})", valid.join(", ")).into());
        }
    }
    if let Some(p) = new_priority {
        let valid = ["low", "medium", "high", "critical"];
        if !valid.contains(&p) {
            return Err(format!("invalid priority '{p}' (valid: {})", valid.join(", ")).into());
        }
    }

    let old_task = store.update_task(id, new_status, new_priority, new_tag)?;

    let mut changes = Vec::new();
    if let Some(s) = new_status {
        changes.push(format!("status: {} -> {s}", old_task.status));
    }
    if let Some(p) = new_priority {
        changes.push(format!("priority: {} -> {p}", old_task.priority));
    }
    if let Some(tag) = new_tag {
        changes.push(format!("tag: {tag}"));
    }

    println!("#{id}: {}", changes.join(", "));
    Ok(())
}

pub fn cmd_done(store: &mut impl TaskStore, id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let title = store.set_task_done(id)?;
    println!("#{id} done: {title}");
    Ok(())
}

pub fn cmd_show(store: &mut impl TaskStore, id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let task = store
        .get_task(id)?
        .ok_or_else(|| format!("task #{id} not found"))?;

    println!("Task #{}: {}", task.id, task.title);
    println!("Status: {}", task.status);
    println!("Priority: {}", task.priority);
    if let Some(ref t) = task.tag {
        println!("Tag: {t}");
    }
    println!("Test Status: {}", task.test_status);
    println!("Created: {}", task.created_at);
    println!("Updated: {}", task.updated_at);
    Ok(())
}

pub fn cmd_test_status(
    store: &mut impl TaskStore,
    id: i64,
    new_test_status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let valid = [
        "planned",
        "designed",
        "implemented",
        "tested",
        "darklaunched",
        "launched",
    ];
    if !valid.contains(&new_test_status) {
        return Err(format!(
            "invalid test status '{new_test_status}' (valid: {})",
            valid.join(", ")
        )
        .into());
    }
    let title = store.set_task_test_status(id, new_test_status)?;
    println!("#{id} test status -> {new_test_status}: {title}");
    Ok(())
}

pub fn cmd_sprint(store: &mut impl TaskStore) -> Result<(), Box<dyn std::error::Error>> {
    let tasks = store.list_sprint_tasks()?;

    if tasks.is_empty() {
        println!("Sprint is clear -- no open or in-progress tasks.");
        return Ok(());
    }

    let mut in_progress = Vec::new();
    let mut blocked = Vec::new();
    let mut open = Vec::new();

    for t in &tasks {
        let tag_str = t
            .tag
            .as_ref()
            .map(|t| format!(" [{t}]"))
            .unwrap_or_default();
        let line = format!(
            "  #{id} ({priority}){tag_str} {title}  [test:{ts}]",
            id = t.id,
            priority = t.priority,
            title = t.title,
            ts = t.test_status,
        );

        match t.status.as_str() {
            "in-progress" => in_progress.push(line),
            "blocked" => blocked.push(line),
            _ => open.push(line),
        }
    }

    if !in_progress.is_empty() {
        println!("In Progress:");
        for l in &in_progress {
            println!("{l}");
        }
        println!();
    }

    if !blocked.is_empty() {
        println!("Blocked:");
        for l in &blocked {
            println!("{l}");
        }
        println!();
    }

    if !open.is_empty() {
        println!("Open:");
        for l in &open {
            println!("{l}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Task;

    struct FakeTaskStore {
        tasks: Vec<Task>,
        next_id: i64,
    }

    impl FakeTaskStore {
        fn new() -> Self {
            Self {
                tasks: Vec::new(),
                next_id: 1,
            }
        }
    }

    impl TaskStore for FakeTaskStore {
        fn add_task(
            &mut self,
            title: &str,
            priority: &str,
            tag: Option<&str>,
        ) -> Result<i64, Box<dyn std::error::Error>> {
            let id = self.next_id;
            self.next_id += 1;
            self.tasks.push(Task {
                id,
                title: title.to_string(),
                status: "open".to_string(),
                priority: priority.to_string(),
                tag: tag.map(|t| t.to_string()),
                test_status: "planned".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            });
            Ok(id)
        }

        fn get_task(&mut self, id: i64) -> Result<Option<Task>, Box<dyn std::error::Error>> {
            Ok(self.tasks.iter().find(|t| t.id == id).cloned())
        }

        fn list_tasks(
            &mut self,
            status_filter: Option<&str>,
            tag_filter: Option<&str>,
            test_status_filter: Option<&str>,
        ) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
            Ok(self
                .tasks
                .iter()
                .filter(|t| status_filter.map_or(true, |s| t.status == s))
                .filter(|t| tag_filter.map_or(true, |tag| t.tag.as_deref() == Some(tag)))
                .filter(|t| test_status_filter.map_or(true, |ts| t.test_status == ts))
                .cloned()
                .collect())
        }

        fn update_task(
            &mut self,
            id: i64,
            new_status: Option<&str>,
            new_priority: Option<&str>,
            new_tag: Option<&str>,
        ) -> Result<Task, Box<dyn std::error::Error>> {
            let t = self
                .tasks
                .iter_mut()
                .find(|t| t.id == id)
                .ok_or_else(|| format!("task #{id} not found"))?;
            let old = t.clone();
            if let Some(s) = new_status {
                t.status = s.to_string();
            }
            if let Some(p) = new_priority {
                t.priority = p.to_string();
            }
            if let Some(tag) = new_tag {
                t.tag = Some(tag.to_string());
            }
            Ok(old)
        }

        fn set_task_done(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>> {
            let t = self
                .tasks
                .iter_mut()
                .find(|t| t.id == id)
                .ok_or_else(|| format!("task #{id} not found"))?;
            t.status = "done".to_string();
            Ok(t.title.clone())
        }

        fn set_task_test_status(
            &mut self,
            id: i64,
            test_status: &str,
        ) -> Result<String, Box<dyn std::error::Error>> {
            let t = self
                .tasks
                .iter_mut()
                .find(|t| t.id == id)
                .ok_or_else(|| format!("task #{id} not found"))?;
            t.test_status = test_status.to_string();
            Ok(t.title.clone())
        }

        fn list_sprint_tasks(&mut self) -> Result<Vec<Task>, Box<dyn std::error::Error>> {
            Ok(self
                .tasks
                .iter()
                .filter(|t| {
                    t.status == "open" || t.status == "in-progress" || t.status == "blocked"
                })
                .cloned()
                .collect())
        }

        fn count_tasks_by_status(
            &mut self,
        ) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error>> {
            let mut counts = std::collections::HashMap::new();
            for t in &self.tasks {
                *counts.entry(t.status.clone()).or_insert(0i64) += 1;
            }
            let mut result: Vec<_> = counts.into_iter().collect();
            result.sort();
            Ok(result)
        }
    }

    #[test]
    fn add_and_query() {
        let mut store = FakeTaskStore::new();
        cmd_add(&mut store, "implement auth", "high", Some("backend")).unwrap();
        assert_eq!(store.tasks.len(), 1);
        assert_eq!(store.tasks[0].title, "implement auth");
        assert_eq!(store.tasks[0].priority, "high");
        assert_eq!(store.tasks[0].tag, Some("backend".to_string()));
    }

    #[test]
    fn invalid_priority_fails() {
        let mut store = FakeTaskStore::new();
        let err = cmd_add(&mut store, "task", "urgent", None).unwrap_err();
        assert!(err.to_string().contains("invalid priority"));
    }

    #[test]
    fn update_status() {
        let mut store = FakeTaskStore::new();
        cmd_add(&mut store, "task one", "medium", None).unwrap();
        cmd_update(&mut store, 1, Some("in-progress"), None, None).unwrap();
        assert_eq!(store.tasks[0].status, "in-progress");
    }

    #[test]
    fn update_nothing_fails() {
        let mut store = FakeTaskStore::new();
        cmd_add(&mut store, "task", "medium", None).unwrap();
        let err = cmd_update(&mut store, 1, None, None, None).unwrap_err();
        assert!(err.to_string().contains("nothing to update"));
    }

    #[test]
    fn done_marks_complete() {
        let mut store = FakeTaskStore::new();
        cmd_add(&mut store, "task", "medium", None).unwrap();
        cmd_done(&mut store, 1).unwrap();
        assert_eq!(store.tasks[0].status, "done");
    }

    #[test]
    fn show_not_found() {
        let mut store = FakeTaskStore::new();
        let err = cmd_show(&mut store, 999).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_status_valid_transition() {
        let mut store = FakeTaskStore::new();
        cmd_add(&mut store, "feature x", "medium", None).unwrap();
        assert_eq!(store.tasks[0].test_status, "planned");
        cmd_test_status(&mut store, 1, "implemented").unwrap();
        assert_eq!(store.tasks[0].test_status, "implemented");
    }

    #[test]
    fn test_status_invalid_value() {
        let mut store = FakeTaskStore::new();
        cmd_add(&mut store, "task", "medium", None).unwrap();
        let err = cmd_test_status(&mut store, 1, "bogus").unwrap_err();
        assert!(err.to_string().contains("invalid test status"));
    }

    #[test]
    fn test_status_not_found() {
        let mut store = FakeTaskStore::new();
        let err = cmd_test_status(&mut store, 999, "tested").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn list_filter_by_test_status() {
        let mut store = FakeTaskStore::new();
        cmd_add(&mut store, "task a", "medium", None).unwrap();
        cmd_add(&mut store, "task b", "medium", None).unwrap();
        cmd_test_status(&mut store, 1, "tested").unwrap();
        let tasks = store.list_tasks(None, None, Some("tested")).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "task a");
    }
}
