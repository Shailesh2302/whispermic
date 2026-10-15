//! OS-level typing at the current cursor (works in ANY app).
//! Uses `enigo` key injection — no clipboard, no focus stealing.

use anyhow::Result;
use enigo::{Enigo, Keyboard, Settings};
use std::sync::Mutex;

pub struct Typer {
    enigo: Mutex<Enigo>,
    pub enabled: bool,
}

impl Typer {
    pub fn new(enabled: bool) -> Self {
        let enigo =
            Enigo::new(&Settings::default()).expect("failed to initialize keyboard output");
        Self { enigo: Mutex::new(enigo), enabled }
    }

    /// Type text char-by-char at cursor. Call per refined partial for realtime feel.
    /// Keeps chunks small (<200 chars) so no app input buffer chokes.
    pub fn type_text(&self, text: &str) -> Result<()> {
        if !self.enabled || text.is_empty() {
            return Ok(());
        }
        let mut e = self.enigo.lock().unwrap();
        // enigo 0.2 `Keyboard::text` handles unicode + newlines
        for chunk in text.chars().collect::<Vec<_>>().chunks(64) {
            let s: String = chunk.iter().collect();
            e.text(&s)
                .map_err(|e| anyhow::anyhow!("keyboard output failed: {e:?}"))?;
        }
        Ok(())
    }
}
