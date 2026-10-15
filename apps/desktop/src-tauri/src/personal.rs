//! Personal dictionary + snippets (WhisperFlow-style, 100% offline).
//! - Dictionary: STT mishears a name/term → auto-correct it. `{"kubernetes": "Kubernetes"}`
//! - Snippets: speak a short trigger → expand saved text. `{"my email": "ada@work.com"}`
//! Stored as plain JSON in ~/.whispermic/ so users can edit by hand too.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".whispermic")
}

fn load_map(name: &str) -> HashMap<String, String> {
    let path = data_dir().join(name);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_map(name: &str, map: &HashMap<String, String>) -> Result<()> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(name), serde_json::to_string_pretty(map)?)?;
    Ok(())
}

// ------------------------------------------------------------- Dictionary

/// Whole-word, case-insensitive correction applied per line (newlines preserved).
pub fn apply_dictionary(s: &str, dict: &HashMap<String, String>) -> String {
    if dict.is_empty() {
        return s.to_string();
    }
    let lower_lookup: HashMap<String, &String> =
        dict.iter().map(|(k, v)| (k.to_lowercase(), v)).collect();
    s.split('\n')
        .map(|line| {
            line.split_whitespace()
                .map(|w| {
                    let (core, pre, post) = split_affixes(w);
                    match lower_lookup.get(&core.to_lowercase()) {
                        Some(repl) => format!("{pre}{repl}{post}"),
                        None => w.to_string(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Split "“Kubernetes,”" → ("Kubernetes", "“", ",\""). Keeps punctuation/case around the core word.
fn split_affixes(w: &str) -> (String, String, String) {
    let start = w.find(|c: char| c.is_alphanumeric()).unwrap_or(w.len());
    let end = w
        .rfind(|c: char| c.is_alphanumeric())
        .map(|i| i + w[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(0))
        .unwrap_or(0);
    if start >= end {
        return (w.to_string(), String::new(), String::new());
    }
    (
        w[start..end].to_string(),
        w[..start].to_string(),
        w[end..].to_string(),
    )
}

pub fn get_dictionary() -> HashMap<String, String> {
    load_map("dictionary.json")
}

pub fn set_word(misheard: String, correction: String) -> Result<HashMap<String, String>> {
    let mut map = load_map("dictionary.json");
    map.insert(misheard, correction);
    save_map("dictionary.json", &map)?;
    Ok(map)
}

pub fn remove_word(misheard: &str) -> Result<HashMap<String, String>> {
    let mut map = load_map("dictionary.json");
    let key = map.keys().find(|k| k.to_lowercase() == misheard.to_lowercase()).cloned();
    if let Some(k) = key {
        map.remove(&k);
        save_map("dictionary.json", &map)?;
    }
    Ok(map)
}

// ------------------------------------------------------------- Snippets

/// Whole-utterance trigger match (case-insensitive, trimmed). Returns expansion or None.
/// Deliberately whole-utterance only: predictable, no mid-sentence surprises.
pub fn expand_snippet(s: &str, snippets: &HashMap<String, String>) -> Option<String> {
    if snippets.is_empty() {
        return None;
    }
    let key = s.trim().to_lowercase();
    snippets
        .iter()
        .find(|(k, _)| k.to_lowercase() == key)
        .map(|(_, v)| v.clone())
}

pub fn get_snippets() -> HashMap<String, String> {
    load_map("snippets.json")
}

pub fn set_snippet(trigger: String, expansion: String) -> Result<HashMap<String, String>> {
    let mut map = load_map("snippets.json");
    map.insert(trigger, expansion);
    save_map("snippets.json", &map)?;
    Ok(map)
}

pub fn remove_snippet(trigger: &str) -> Result<HashMap<String, String>> {
    let mut map = load_map("snippets.json");
    let key = map.keys().find(|k| k.to_lowercase() == trigger.to_lowercase()).cloned();
    if let Some(k) = key {
        map.remove(&k);
        save_map("snippets.json", &map)?;
    }
    Ok(map)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub from: String,
    pub to: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dict_corrects_case_insensitive_with_punct() {
        let mut d = HashMap::new();
        d.insert("kubernetes".into(), "Kubernetes".into());
        assert_eq!(apply_dictionary("i love KUBERNETES,", &d), "i love Kubernetes,");
    }
    #[test]
    fn snippet_expands_whole_utterance_only() {
        let mut s = HashMap::new();
        s.insert("my email".into(), "ada@work.com".into());
        assert_eq!(expand_snippet("  My Email ", &s), Some("ada@work.com".into()));
        assert_eq!(expand_snippet("my email is x", &s), None);
    }
}
