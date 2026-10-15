//! Instant text refinement (<1ms, offline, no network).
//! Runs on every STT partial BEFORE typing so output feels clean in real time:
//! fillers stripped, caps + punctuation restored, spacing fixed, voice commands mapped.

const FILLERS: &[&str] = &["um", "uh", "like", "you know", "basically", "actually"];

/// Polish raw STT text for immediate typing.
pub fn refine(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.is_empty() {
        return s;
    }
    s = apply_voice_commands(&s);
    s = strip_fillers(&s);
    s = collapse_spaces(&s);
    s = fix_punct_spacing(&s);
    s = capitalize_sentences(&s);
    ensure_trailing_space(s)
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
}
