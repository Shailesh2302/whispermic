//! Instant text refinement (<1ms, offline, no network).
//! Runs on every STT partial BEFORE typing so output feels clean in real time:
//! fillers stripped, caps + punctuation restored, spacing fixed, voice commands
//! mapped, personal dictionary applied. Snippets expand on finished utterances.
//! Writing style tunes the polish: professional / casual / concise.

use std::collections::HashMap;

const FILLERS: &[&str] = &["um", "uh", "like", "you know", "basically", "actually"];
const HEDGES: &[&[&str]] = &[
    &["i", "think"],
    &["just"],
    &["really"],
    &["very"],
    &["kind", "of"],
    &["sort", "of"],
    &["pretty", "much"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Professional,
    Casual,
    Concise,
}

impl Style {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "casual" => Style::Casual,
            "concise" => Style::Concise,
            _ => Style::Professional,
        }
    }
}

/// Polish a streaming partial (snippets NOT expanded — utterance isn't complete).
pub fn refine_partial(raw: &str, style: Style, dict: &HashMap<String, String>) -> String {
    polish(raw, style, dict)
}

/// Polish a finished utterance. Whole-utterance snippet triggers expand here.
pub fn refine_final(
    raw: &str,
    style: Style,
    dict: &HashMap<String, String>,
    snippets: &HashMap<String, String>,
) -> String {
    if let Some(expanded) = super::personal::expand_snippet(raw, snippets) {
        return ensure_trailing_space(expanded.trim().to_string());
    }
    polish(raw, style, dict)
}

/// Polish raw STT text for immediate typing (default style, no dictionary).
pub fn refine(raw: &str) -> String {
    polish(raw, Style::Professional, &HashMap::new())
}

fn polish(raw: &str, style: Style, dict: &HashMap<String, String>) -> String {
    let mut s = raw.trim().to_string();
    if s.is_empty() {
        return s;
    }
    s = apply_voice_commands(&s);
    s = strip_fillers(&s);
    if style == Style::Concise {
        s = strip_hedges(&s);
    }
    s = super::personal::apply_dictionary(&s, dict);
    s = collapse_spaces(&s);
    s = fix_punct_spacing(&s);
    if style != Style::Casual {
        s = capitalize_sentences(&s);
    }
    ensure_trailing_space(s)
}

fn strip_hedges(s: &str) -> String {
    s.split('\n')
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let mut out: Vec<&str> = Vec::with_capacity(words.len());
            let mut i = 0;
            while i < words.len() {
                let mut skipped = false;
                for h in HEDGES {
                    if words.len() - i >= h.len()
                        && words[i..i + h.len()]
                            .iter()
                            .zip(h.iter())
                            .all(|(w, hw)| w.to_lowercase() == **hw)
                    {
                        i += h.len();
                        skipped = true;
                        break;
                    }
                }
                if !skipped {
                    out.push(words[i]);
                    i += 1;
                }
            }
            out.join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn apply_voice_commands(s: &str) -> String {
    let mut s = s.to_string();
    // order matters: multi-word first
    let map = [
        ("new line", "\n"),
        ("new paragraph", "\n\n"),
        ("full stop", "."),
        ("period", "."),
        ("question mark", "?"),
        ("exclamation mark", "!"),
        ("comma", ","),
        ("colon", ":"),
        ("semicolon", ";"),
        (" tab ", " "),
    ];
    // case-insensitive replace via lowercase scan (cheap, chunk-sized strings).
    // NOTE: `lower` is recomputed per phrase — earlier replacements change
    // string length, so a single precomputed index map would misalign bytes.
    for (k, v) in map {
        // naive but fast enough for <200 char partials
        let lower = s.to_lowercase();
        let mut out = String::with_capacity(s.len());
        let mut rest = lower.as_str();
        let mut orig_rest = s.as_str();
        // simple loop: find in lower, splice same byte range in original
        loop {
            match rest.find(k) {
                Some(i) => {
                    out.push_str(&orig_rest[..i]);
                    out.push_str(v);
                    rest = &rest[i + k.len()..];
                    orig_rest = &orig_rest[i + k.len()..];
                }
                None => {
                    out.push_str(orig_rest);
                    break;
                }
            }
        }
        s = out;
    }
    s
}

fn strip_fillers(s: &str) -> String {
    // Preserve newlines from voice commands: filter words per line.
    s.split('\n')
        .map(|line| {
            line.split_whitespace()
                .filter(|w| {
                    let lw = w.to_lowercase();
                    let clean = lw.trim_matches(|c: char| !c.is_alphanumeric());
                    !FILLERS.contains(&clean)
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collapse_spaces(s: &str) -> String {
    // Preserve newlines from voice commands ("new line"/"new paragraph"):
    // collapse whitespace per line, then rejoin. Blank lines (paragraphs) survive.
    s.split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn fix_punct_spacing(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == ' ' {
            if let Some(&n) = chars.get(i + 1) {
                if matches!(n, ',' | '.' | '?' | '!' | ':' | ';' | ')') {
                    continue; // drop space before punctuation
                }
            }
            // avoid double space
            if out.ends_with(' ') || out.ends_with('\n') {
                continue;
            }
        }
        out.push(c);
    }
    out
}

fn capitalize_sentences(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cap_next = true;
    for c in s.chars() {
        if cap_next && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            cap_next = false;
        } else {
            out.push(c);
        }
        if matches!(c, '.' | '?' | '!' | '\n') {
            cap_next = true;
        }
    }
    // lone "i" -> "I" is handled by capitalize pass above for sentence starts;
    // mid-sentence " i " is intentionally left to STT casing to avoid false hits.
    out
}

fn ensure_trailing_space(mut s: String) -> String {
    // Partials are typed incrementally; a trailing space separates words.
    // Final punctuation still gets a space so next partial doesn't glue.
    if !s.ends_with(' ') && !s.ends_with('\n') {
        s.push(' ');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn removes_fillers_and_caps() {
        assert_eq!(refine("um hello world"), "Hello world ");
    }
    #[test]
    fn voice_commands() {
        // newline starts a new sentence → capitalised
        assert_eq!(refine("hello new line world"), "Hello\nWorld ");
    }
    #[test]
    fn concise_strips_hedges() {
        let d = HashMap::new();
        assert_eq!(
            refine_partial("i think we should just ship it", Style::Concise, &d),
            "We should ship it "
        );
    }
    #[test]
    fn casual_skips_caps() {
        let d = HashMap::new();
        assert_eq!(
            refine_partial("hello world", Style::Casual, &d),
            "hello world "
        );
    }
    #[test]
    fn dictionary_corrects_names() {
        let mut d = HashMap::new();
        d.insert("kubernetes".into(), "Kubernetes".into());
        assert_eq!(
            refine_partial("i love kubernetes", Style::Professional, &d),
            "I love Kubernetes "
        );
    }
    #[test]
    fn final_expands_snippet() {
        let d = HashMap::new();
        let mut s = HashMap::new();
        s.insert("my email".into(), "ada@work.com".into());
        assert_eq!(
            refine_final("my email", Style::Professional, &d, &s),
            "ada@work.com "
        );
        // partials never expand mid-utterance
        assert_eq!(refine_partial("my email", Style::Professional, &d), "My email ");
    }
}
