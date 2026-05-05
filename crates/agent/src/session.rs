use anyhow::Result;
use directories::ProjectDirs;
use nxc_provider::Message;
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub cwd:      String,
    pub messages: Vec<Message>,
    pub saved_at: u64,
}

fn sessions_dir() -> Option<PathBuf> {
    ProjectDirs::from("dev", "nexuscode", "nxc").map(|d| d.data_dir().join("sessions"))
}

pub fn save_session(messages: &[Message]) -> Result<PathBuf> {
    let dir = sessions_dir().ok_or_else(|| anyhow::anyhow!("no data dir"))?;
    fs::create_dir_all(&dir)?;
    let cwd = std::env::current_dir().unwrap_or_default().display().to_string();
    let ts  = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let session = Session { cwd, messages: messages.to_vec(), saved_at: ts };
    let path = dir.join(format!("{ts}.json"));
    fs::write(&path, serde_json::to_string_pretty(&session)?)?;
    Ok(path)
}

pub fn load_latest_session() -> Result<Option<Vec<Message>>> {
    let dir = sessions_dir().ok_or_else(|| anyhow::anyhow!("no data dir"))?;
    if !dir.exists() { return Ok(None); }
    let mut entries: Vec<_> = fs::read_dir(&dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    match entries.last() {
        None    => Ok(None),
        Some(e) => Ok(Some(load_from_path(&e.path())?)),
    }
}

pub fn load_from_path(path: &Path) -> Result<Vec<Message>> {
    let text = fs::read_to_string(path)?;
    let s: Session = serde_json::from_str(&text)?;
    Ok(s.messages)
}

pub fn save_to_path(messages: &[Message], path: &Path) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_default().display().to_string();
    let ts  = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let s   = Session { cwd, messages: messages.to_vec(), saved_at: ts };
    fs::write(path, serde_json::to_string_pretty(&s)?)?;
    Ok(())
}

pub fn list_sessions() -> Result<Vec<PathBuf>> {
    let dir = sessions_dir().ok_or_else(|| anyhow::anyhow!("no data dir"))?;
    if !dir.exists() { return Ok(vec![]); }
    let mut entries: Vec<_> = fs::read_dir(&dir)?.filter_map(|e| e.ok())
        .map(|e| e.path()).collect();
    entries.sort();
    entries.reverse();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxc_provider::Message;
    use tempfile::tempdir;

    #[test]
    fn save_and_load_round_trips() {
        let dir  = tempdir().unwrap();
        let msgs = vec![Message::user("hello"), Message::assistant_text("hi")];
        let path = dir.path().join("session.json");
        save_to_path(&msgs, &path).unwrap();
        let loaded = load_from_path(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].role, "user");
    }
}
