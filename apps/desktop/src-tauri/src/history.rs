//! Local dictation history hub (offline).
//! Every finished utterance is appended to ~/.whispermic/history.jsonl
//! (newest last, capped at 200). Overlay lists recent entries; clicking one
//! re-types it at the cursor. No cloud, no account.

use anyhow::Result;
use serde::{Deserialize, Serialize};

const MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub t: String,
    pub text: String,
}

fn data_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(home).join(".whispermic")
}

fn path() -> std::path::PathBuf {
    data_dir().join("history.jsonl")
}

pub fn append(text: &str) -> Result<()> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let mut items = list();
    // skip exact repeat of the last entry (partials merging on stop)
    if items.last().map(|i| i.text.as_str()) == Some(text) {
        return Ok(());
    }
    items.push(HistoryItem {
        t: chrono_now(),
        text: text.to_string(),
    });
    if items.len() > MAX_ENTRIES {
        items.drain(..items.len() - MAX_ENTRIES);
    }
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let out = items
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    std::fs::write(path(), out)?;
    Ok(())
}

pub fn list() -> Vec<HistoryItem> {
    std::fs::read_to_string(path())
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

pub fn clear() -> Result<()> {
    let p = path();
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

fn chrono_now() -> String {
    // std-only timestamp (no chrono dep): secs since epoch, human enough for a hub list.
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
