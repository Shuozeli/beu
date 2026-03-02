use crate::store::JournalStore;
use crate::time_helper;

pub fn cmd_open(store: &mut impl JournalStore) -> Result<(), Box<dyn std::error::Error>> {
    let session_id = time_helper::generate_id("j");
    store.create_session(&session_id)?;
    println!("Session {session_id} opened.");
    Ok(())
}

pub fn cmd_log(
    store: &mut impl JournalStore,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_id = get_open_session(store)?;
    let entry_id = time_helper::generate_id("e");
    store.add_entry(&session_id, &entry_id, message, None)?;
    println!("Logged: {message}");
    Ok(())
}

pub fn cmd_note(
    store: &mut impl JournalStore,
    tag: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_id = get_open_session(store)?;
    let entry_id = time_helper::generate_id("e");
    store.add_entry(&session_id, &entry_id, message, Some(tag))?;
    println!("[{tag}] {message}");
    Ok(())
}

pub fn cmd_summary(store: &mut impl JournalStore) -> Result<(), Box<dyn std::error::Error>> {
    let session_id = get_open_session(store)?;

    let session = store.get_session(&session_id)?.ok_or("session not found")?;

    let entries = store.list_entries(&session_id)?;

    println!("Session: {}", session.id);
    println!("Status: {}", session.status);
    println!("Started: {}", session.started_at);
    if let Some(ref closed) = session.closed_at {
        println!("Closed: {closed}");
    }
    println!();

    if entries.is_empty() {
        println!("(no entries yet)");
    } else {
        for entry in &entries {
            if let Some(ref t) = entry.tag {
                println!("  [{t}] {}: {}", entry.created_at, entry.message);
            } else {
                println!("  {}: {}", entry.created_at, entry.message);
            }
        }
    }

    Ok(())
}

pub fn cmd_close(store: &mut impl JournalStore) -> Result<(), Box<dyn std::error::Error>> {
    let session_id = get_open_session(store)?;
    store.close_session(&session_id)?;
    println!("Session {session_id} closed.");
    Ok(())
}

fn get_open_session(store: &mut impl JournalStore) -> Result<String, Box<dyn std::error::Error>> {
    store
        .get_open_session_id()?
        .ok_or_else(|| "no open session (run 'beu journal open' first)".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{JournalEntry, Session};
    use std::collections::HashMap;

    struct FakeJournalStore {
        sessions: Vec<Session>,
        entries: HashMap<String, Vec<JournalEntry>>,
    }

    impl FakeJournalStore {
        fn new() -> Self {
            Self {
                sessions: Vec::new(),
                entries: HashMap::new(),
            }
        }
    }

    impl JournalStore for FakeJournalStore {
        fn create_session(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
            self.sessions.push(Session {
                id: session_id.to_string(),
                started_at: "2024-01-01T00:00:00Z".to_string(),
                closed_at: None,
                status: "open".to_string(),
            });
            Ok(())
        }

        fn get_open_session_id(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
            Ok(self
                .sessions
                .iter()
                .rev()
                .find(|s| s.status == "open")
                .map(|s| s.id.clone()))
        }

        fn get_session(
            &mut self,
            session_id: &str,
        ) -> Result<Option<Session>, Box<dyn std::error::Error>> {
            Ok(self.sessions.iter().find(|s| s.id == session_id).cloned())
        }

        fn close_session(&mut self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
            if let Some(s) = self.sessions.iter_mut().find(|s| s.id == session_id) {
                s.status = "closed".to_string();
            }
            Ok(())
        }

        fn add_entry(
            &mut self,
            session_id: &str,
            _entry_id: &str,
            message: &str,
            tag: Option<&str>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.entries
                .entry(session_id.to_string())
                .or_default()
                .push(JournalEntry {
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    message: message.to_string(),
                    tag: tag.map(|t| t.to_string()),
                });
            Ok(())
        }

        fn list_entries(
            &mut self,
            session_id: &str,
        ) -> Result<Vec<JournalEntry>, Box<dyn std::error::Error>> {
            Ok(self.entries.get(session_id).cloned().unwrap_or_default())
        }
    }

    #[test]
    fn open_creates_session() {
        let mut store = FakeJournalStore::new();
        cmd_open(&mut store).unwrap();
        assert_eq!(store.sessions.len(), 1);
        assert_eq!(store.sessions[0].status, "open");
    }

    #[test]
    fn log_requires_open_session() {
        let mut store = FakeJournalStore::new();
        let err = cmd_log(&mut store, "hello").unwrap_err();
        assert!(err.to_string().contains("no open session"));
    }

    #[test]
    fn log_and_close() {
        let mut store = FakeJournalStore::new();
        cmd_open(&mut store).unwrap();
        cmd_log(&mut store, "test message").unwrap();
        cmd_close(&mut store).unwrap();

        let sid = &store.sessions[0].id;
        assert_eq!(store.sessions[0].status, "closed");
        let entries = &store.entries[sid];
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "test message");
    }

    #[test]
    fn note_with_tag() {
        let mut store = FakeJournalStore::new();
        cmd_open(&mut store).unwrap();
        cmd_note(&mut store, "decision", "use postgres").unwrap();

        let sid = &store.sessions[0].id;
        let entries = &store.entries[sid];
        assert_eq!(entries[0].tag, Some("decision".to_string()));
        assert_eq!(entries[0].message, "use postgres");
    }

    #[test]
    fn close_then_log_fails() {
        let mut store = FakeJournalStore::new();
        cmd_open(&mut store).unwrap();
        cmd_close(&mut store).unwrap();
        let err = cmd_log(&mut store, "after close").unwrap_err();
        assert!(err.to_string().contains("no open session"));
    }
}
