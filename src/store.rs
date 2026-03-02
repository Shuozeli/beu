//! Per-module store traits and domain types.
//!
//! Command files depend only on these traits -- they have zero knowledge of SQL
//! or any other storage mechanism. Implementations live in `sqlite/`.

// ---------------------------------------------------------------------------
// Artifact
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Artifact {
    pub name: String,
    pub artifact_type: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ArtifactChangelog {
    pub message: String,
    pub created_at: String,
}

pub trait ArtifactStore {
    fn add_artifact(
        &mut self,
        name: &str,
        artifact_type: &str,
        description: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn get_artifact(&mut self, name: &str) -> Result<Option<Artifact>, Box<dyn std::error::Error>>;

    fn update_artifact_status(
        &mut self,
        name: &str,
        new_status: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;

    fn describe_artifact(
        &mut self,
        name: &str,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn list_artifacts(
        &mut self,
        status_filter: Option<&str>,
    ) -> Result<Vec<Artifact>, Box<dyn std::error::Error>>;

    fn remove_artifact(&mut self, name: &str) -> Result<bool, Box<dyn std::error::Error>>;

    fn add_changelog_entry(
        &mut self,
        name: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn list_changelog(
        &mut self,
        name: &str,
    ) -> Result<Vec<ArtifactChangelog>, Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub tag: Option<String>,
    pub test_status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub trait TaskStore {
    fn add_task(
        &mut self,
        title: &str,
        priority: &str,
        tag: Option<&str>,
    ) -> Result<i64, Box<dyn std::error::Error>>;

    fn get_task(&mut self, id: i64) -> Result<Option<Task>, Box<dyn std::error::Error>>;

    fn list_tasks(
        &mut self,
        status_filter: Option<&str>,
        tag_filter: Option<&str>,
        test_status_filter: Option<&str>,
    ) -> Result<Vec<Task>, Box<dyn std::error::Error>>;

    /// Returns the old task state before the update.
    fn update_task(
        &mut self,
        id: i64,
        new_status: Option<&str>,
        new_priority: Option<&str>,
        new_tag: Option<&str>,
    ) -> Result<Task, Box<dyn std::error::Error>>;

    /// Returns the task title.
    fn set_task_done(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>>;

    /// Returns the task title.
    fn set_task_test_status(
        &mut self,
        id: i64,
        test_status: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;

    fn list_sprint_tasks(&mut self) -> Result<Vec<Task>, Box<dyn std::error::Error>>;

    fn count_tasks_by_status(&mut self) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// Idea
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Idea {
    pub id: i64,
    pub title: String,
    pub area: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
}

pub trait IdeaStore {
    fn add_idea(
        &mut self,
        title: &str,
        area: &str,
        priority: &str,
    ) -> Result<i64, Box<dyn std::error::Error>>;

    fn get_idea(&mut self, id: i64) -> Result<Option<Idea>, Box<dyn std::error::Error>>;

    fn list_ideas(
        &mut self,
        area: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<Idea>, Box<dyn std::error::Error>>;

    /// Returns the idea title.
    fn set_idea_done(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>>;

    /// Returns the idea title.
    fn archive_idea(&mut self, id: i64) -> Result<String, Box<dyn std::error::Error>>;

    fn describe_idea(
        &mut self,
        id: i64,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn count_ideas_by_status(&mut self) -> Result<Vec<(String, i64)>, Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// Journal
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Session {
    pub id: String,
    pub started_at: String,
    pub closed_at: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct JournalEntry {
    pub created_at: String,
    pub message: String,
    pub tag: Option<String>,
}

pub trait JournalStore {
    fn create_session(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>>;

    fn get_open_session_id(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>>;

    fn get_session(
        &mut self,
        session_id: &str,
    ) -> Result<Option<Session>, Box<dyn std::error::Error>>;

    fn close_session(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>>;

    fn add_entry(
        &mut self,
        session_id: &str,
        entry_id: &str,
        message: &str,
        tag: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn list_entries(
        &mut self,
        session_id: &str,
    ) -> Result<Vec<JournalEntry>, Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StateEntry {
    pub category: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

pub trait StateStore {
    fn set(
        &mut self,
        category: &str,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn get_by_key(&mut self, key: &str) -> Result<Option<StateEntry>, Box<dyn std::error::Error>>;

    fn list_entries(
        &mut self,
        category: Option<&str>,
    ) -> Result<Vec<StateEntry>, Box<dyn std::error::Error>>;

    fn remove(&mut self, key: &str) -> Result<bool, Box<dyn std::error::Error>>;

    fn clear_category(&mut self, category: &str) -> Result<u64, Box<dyn std::error::Error>>;

    fn count_by_category(&mut self, category: &str) -> Result<i64, Box<dyn std::error::Error>>;

    fn get_checkpoint(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>>;

    fn clear_checkpoint(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    fn list_blockers(&mut self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>>;

    fn list_focus_items(&mut self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// Debug
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DebugSession {
    pub slug: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct DebugEntry {
    pub entry_type: String,
    pub content: String,
    pub created_at: String,
}

pub trait DebugStore {
    fn create_session(&mut self, slug: &str, title: &str)
        -> Result<(), Box<dyn std::error::Error>>;

    fn slug_exists(&mut self, slug: &str) -> Result<bool, Box<dyn std::error::Error>>;

    fn get_session(
        &mut self,
        slug: &str,
    ) -> Result<Option<DebugSession>, Box<dyn std::error::Error>>;

    fn add_entry(
        &mut self,
        slug: &str,
        entry_type: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn update_status(&mut self, slug: &str, status: &str)
        -> Result<(), Box<dyn std::error::Error>>;

    fn update_timestamp(&mut self, slug: &str) -> Result<(), Box<dyn std::error::Error>>;

    fn list_sessions(
        &mut self,
        status_filter: Option<&str>,
    ) -> Result<Vec<DebugSession>, Box<dyn std::error::Error>>;

    fn list_entries(&mut self, slug: &str) -> Result<Vec<DebugEntry>, Box<dyn std::error::Error>>;

    fn count_active(&mut self) -> Result<i64, Box<dyn std::error::Error>>;
}

// ---------------------------------------------------------------------------
// EventLog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Event {
    pub id: i64,
    pub timestamp: String,
    pub module: String,
    pub command: String,
    pub args: String,
    pub status: String,
    pub duration_ms: i64,
}

pub trait EventLogStore {
    fn log_event(
        &mut self,
        module: &str,
        command: &str,
        args: &str,
        status: &str,
        duration_ms: i64,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn recent_events(
        &mut self,
        limit: usize,
        module_filter: Option<&str>,
    ) -> Result<Vec<Event>, Box<dyn std::error::Error>>;

    fn count_events(&mut self) -> Result<i64, Box<dyn std::error::Error>>;

    /// Count mutation events (successful writes in non-artifact, non-system modules)
    /// that occurred after the given ISO 8601 timestamp.
    fn count_mutation_events_since(
        &mut self,
        since: &str,
    ) -> Result<i64, Box<dyn std::error::Error>>;
}
