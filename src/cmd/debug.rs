use crate::store::DebugStore;

const VALID_STATUSES: &[&str] = &["investigating", "root-cause-found", "blocked", "resolved"];

pub fn cmd_open(
    store: &mut impl DebugStore,
    title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_slug = generate_slug(title);
    let slug = find_unique_slug(store, &base_slug)?;
    store.create_session(&slug, title)?;
    println!("Debug session '{slug}' opened.");
    Ok(())
}

pub fn cmd_log(
    store: &mut impl DebugStore,
    slug: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    verify_session_active(store, slug)?;
    store.add_entry(slug, "evidence", message)?;
    store.update_timestamp(slug)?;
    println!("[{slug}] evidence: {message}");
    Ok(())
}

pub fn cmd_symptom(
    store: &mut impl DebugStore,
    slug: &str,
    description: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    verify_session_active(store, slug)?;
    store.add_entry(slug, "symptom", description)?;
    store.update_timestamp(slug)?;
    println!("[{slug}] symptom: {description}");
    Ok(())
}

pub fn cmd_cause(
    store: &mut impl DebugStore,
    slug: &str,
    description: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    verify_session_exists(store, slug)?;
    store.add_entry(slug, "cause", description)?;
    store.update_status(slug, "root-cause-found")?;
    println!("[{slug}] root cause: {description}");
    Ok(())
}

pub fn cmd_resolve(
    store: &mut impl DebugStore,
    slug: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = store
        .get_session(slug)?
        .ok_or_else(|| format!("debug session '{slug}' not found"))?;

    if session.status == "resolved" {
        return Err(format!("debug session '{slug}' is already resolved").into());
    }

    store.update_status(slug, "resolved")?;
    println!("Debug session '{slug}' resolved.");
    Ok(())
}

pub fn cmd_list(
    store: &mut impl DebugStore,
    status: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(s) = status {
        if !VALID_STATUSES.contains(&s) {
            return Err(format!(
                "invalid status '{s}' (valid: {})",
                VALID_STATUSES.join(", ")
            )
            .into());
        }
    }

    let sessions = store.list_sessions(status)?;

    if sessions.is_empty() {
        println!("No debug sessions found. Use 'beu debug open <title>' to start one.");
        return Ok(());
    }

    for s in &sessions {
        println!(
            "  [{status}] {slug}: {title}",
            status = s.status,
            slug = s.slug,
            title = s.title
        );
    }

    Ok(())
}

pub fn cmd_show(store: &mut impl DebugStore, slug: &str) -> Result<(), Box<dyn std::error::Error>> {
    let session = store
        .get_session(slug)?
        .ok_or_else(|| format!("debug session '{slug}' not found"))?;

    println!("Debug: {slug} ({})", session.status);
    println!("Title: {}", session.title);
    println!("Created: {}", session.created_at);
    println!("Updated: {}", session.updated_at);

    let entries = store.list_entries(slug)?;

    if entries.is_empty() {
        println!("\n(no entries yet)");
    } else {
        println!("\nTimeline:");
        for e in &entries {
            println!(
                "  [{entry_type}] {time}: {content}",
                entry_type = e.entry_type,
                time = e.created_at,
                content = e.content
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_slug(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    let mut result = String::new();
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash && !result.is_empty() {
                result.push('-');
            }
            prev_dash = true;
        } else {
            result.push(c);
            prev_dash = false;
        }
    }

    if result.ends_with('-') {
        result.pop();
    }

    if result.len() > 50 {
        if let Some(pos) = result[..50].rfind('-') {
            result.truncate(pos);
        } else {
            result.truncate(50);
        }
    }

    result
}

fn find_unique_slug(
    store: &mut impl DebugStore,
    base_slug: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if !store.slug_exists(base_slug)? {
        return Ok(base_slug.to_string());
    }

    for n in 2..=100 {
        let candidate = format!("{base_slug}-{n}");
        if !store.slug_exists(&candidate)? {
            return Ok(candidate);
        }
    }

    Err("could not generate unique slug after 100 attempts".into())
}

fn verify_session_exists(
    store: &mut impl DebugStore,
    slug: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if store.get_session(slug)?.is_none() {
        return Err(format!("debug session '{slug}' not found").into());
    }
    Ok(())
}

fn verify_session_active(
    store: &mut impl DebugStore,
    slug: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = store
        .get_session(slug)?
        .ok_or_else(|| format!("debug session '{slug}' not found"))?;

    if session.status == "resolved" {
        return Err(format!("debug session '{slug}' is resolved; cannot add entries").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{DebugEntry, DebugSession};

    struct FakeDebugStore {
        sessions: Vec<DebugSession>,
        entries: Vec<DebugEntry>,
        entry_slugs: Vec<String>,
    }

    impl FakeDebugStore {
        fn new() -> Self {
            Self {
                sessions: Vec::new(),
                entries: Vec::new(),
                entry_slugs: Vec::new(),
            }
        }
    }

    impl DebugStore for FakeDebugStore {
        fn create_session(
            &mut self,
            slug: &str,
            title: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.sessions.push(DebugSession {
                slug: slug.to_string(),
                title: title.to_string(),
                status: "investigating".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            });
            Ok(())
        }

        fn slug_exists(&mut self, slug: &str) -> Result<bool, Box<dyn std::error::Error>> {
            Ok(self.sessions.iter().any(|s| s.slug == slug))
        }

        fn get_session(
            &mut self,
            slug: &str,
        ) -> Result<Option<DebugSession>, Box<dyn std::error::Error>> {
            Ok(self.sessions.iter().find(|s| s.slug == slug).cloned())
        }

        fn add_entry(
            &mut self,
            slug: &str,
            entry_type: &str,
            content: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.entries.push(DebugEntry {
                entry_type: entry_type.to_string(),
                content: content.to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            });
            self.entry_slugs.push(slug.to_string());
            Ok(())
        }

        fn update_status(
            &mut self,
            slug: &str,
            status: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            if let Some(s) = self.sessions.iter_mut().find(|s| s.slug == slug) {
                s.status = status.to_string();
            }
            Ok(())
        }

        fn update_timestamp(&mut self, _slug: &str) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }

        fn list_sessions(
            &mut self,
            status_filter: Option<&str>,
        ) -> Result<Vec<DebugSession>, Box<dyn std::error::Error>> {
            Ok(self
                .sessions
                .iter()
                .filter(|s| status_filter.map_or(true, |f| s.status == f))
                .cloned()
                .collect())
        }

        fn list_entries(
            &mut self,
            slug: &str,
        ) -> Result<Vec<DebugEntry>, Box<dyn std::error::Error>> {
            Ok(self
                .entries
                .iter()
                .zip(self.entry_slugs.iter())
                .filter(|(_, s)| s.as_str() == slug)
                .map(|(e, _)| e.clone())
                .collect())
        }

        fn count_active(&mut self) -> Result<i64, Box<dyn std::error::Error>> {
            Ok(self
                .sessions
                .iter()
                .filter(|s| s.status != "resolved")
                .count() as i64)
        }
    }

    #[test]
    fn open_creates_session() {
        let mut store = FakeDebugStore::new();
        cmd_open(&mut store, "test issue").unwrap();
        assert_eq!(store.sessions.len(), 1);
        assert_eq!(store.sessions[0].slug, "test-issue");
        assert_eq!(store.sessions[0].status, "investigating");
    }

    #[test]
    fn log_evidence() {
        let mut store = FakeDebugStore::new();
        cmd_open(&mut store, "bug").unwrap();
        cmd_log(&mut store, "bug", "found stack trace").unwrap();
        assert_eq!(store.entries.len(), 1);
        assert_eq!(store.entries[0].entry_type, "evidence");
    }

    #[test]
    fn cause_sets_root_cause_found() {
        let mut store = FakeDebugStore::new();
        cmd_open(&mut store, "crash").unwrap();
        cmd_cause(&mut store, "crash", "null pointer").unwrap();
        assert_eq!(store.sessions[0].status, "root-cause-found");
    }

    #[test]
    fn resolve_marks_resolved() {
        let mut store = FakeDebugStore::new();
        cmd_open(&mut store, "fix").unwrap();
        cmd_resolve(&mut store, "fix").unwrap();
        assert_eq!(store.sessions[0].status, "resolved");
    }

    #[test]
    fn resolve_already_resolved_fails() {
        let mut store = FakeDebugStore::new();
        cmd_open(&mut store, "done").unwrap();
        cmd_resolve(&mut store, "done").unwrap();
        let err = cmd_resolve(&mut store, "done").unwrap_err();
        assert!(err.to_string().contains("already resolved"));
    }

    #[test]
    fn log_on_resolved_fails() {
        let mut store = FakeDebugStore::new();
        cmd_open(&mut store, "old").unwrap();
        cmd_resolve(&mut store, "old").unwrap();
        let err = cmd_log(&mut store, "old", "more info").unwrap_err();
        assert!(err.to_string().contains("resolved"));
    }

    #[test]
    fn show_not_found() {
        let mut store = FakeDebugStore::new();
        let err = cmd_show(&mut store, "nonexistent").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn invalid_status_filter() {
        let mut store = FakeDebugStore::new();
        let err = cmd_list(&mut store, Some("invalid")).unwrap_err();
        assert!(err.to_string().contains("invalid status"));
    }
}
