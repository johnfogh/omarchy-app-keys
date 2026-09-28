use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::chord::press_chord;

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Chord(String),
    Text(Vec<u8>),
}

pub fn parse_sequence(bytes: &[u8]) -> Result<Vec<Action>, String> {
    let mut actions = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        if rest.starts_with(b"C ") {
            let line_end = rest.iter().position(|byte| *byte == b'\n').ok_or("truncated chord")?;
            let chord = std::str::from_utf8(&rest[2..line_end]).map_err(|_| "chord is not utf-8")?;
            if chord.is_empty() {
                return Err("empty chord".into());
            }
            actions.push(Action::Chord(chord.to_string()));
            i += line_end + 1;
        } else if rest.starts_with(b"T ") {
            let line_end = rest.iter().position(|byte| *byte == b'\n').ok_or("truncated text length")?;
            let length: usize = std::str::from_utf8(&rest[2..line_end])
                .map_err(|_| "text length is not utf-8")?
                .parse()
                .map_err(|_| "text length is not a number")?;
            let start = i + line_end + 1;
            let end = start + length;
            if end > bytes.len() {
                return Err("truncated text".into());
            }
            actions.push(Action::Text(bytes[start..end].to_vec()));
            i = end;
        } else if rest.starts_with(b"\n") {
            i += 1;
        } else {
            return Err("unknown sequence record".into());
        }
    }
    Ok(actions)
}

fn uses_super(shortcut: &str) -> bool {
    shortcut.split('+').any(|part| part == "super")
}

fn hypr_shortcut(shortcut: &str) -> Result<(String, String), String> {
    let mut parts: Vec<&str> = shortcut.split('+').filter(|part| !part.is_empty()).collect();
    let key = parts.pop().ok_or("empty chord")?;
    let key = match key {
        "-" => "minus",
        "=" => "equal",
        other => other,
    };
    let mods = parts
        .iter()
        .map(|part| part.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join(" + ");
    Ok((mods, key.to_string()))
}

fn hypr_dispatch(request: &str) -> Result<(), String> {
    let status = Command::new("hyprctl")
        .args(["dispatch", request])
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("hyprctl failed: {request}"))
    }
}

fn type_text(text: &[u8]) -> Result<(), String> {
    let mut child = Command::new("wtype")
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|err| err.to_string())?;
    child
        .stdin
        .as_mut()
        .ok_or("wtype stdin")?
        .write_all(text)
        .map_err(|err| err.to_string())?;
    let status = child.wait().map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("wtype failed".into())
    }
}

/// Focus `window` and send every action, in order.
pub fn send_sequence(window: &str, path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    let actions = parse_sequence(&bytes)?;
    let window = if window.starts_with("address:") {
        window.to_string()
    } else {
        format!("address:{window}")
    };
    hypr_dispatch(&format!("hl.dsp.focus({{ window = {window:?} }})"))?;
    for action in actions {
        match action {
            Action::Text(text) => type_text(&text)?,
            Action::Chord(shortcut) if uses_super(&shortcut) => press_chord(&shortcut)?,
            Action::Chord(shortcut) => {
                let (mods, key) = hypr_shortcut(&shortcut)?;
                hypr_dispatch(&format!(
                    "hl.dsp.send_shortcut({{ mods = {mods:?}, key = {key:?}, window = {window:?} }})"
                ))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_sequence, Action};

    #[test]
    fn sequence_keeps_chords_and_strings_in_order() {
        let bytes = b"C ctrl+f\nT 5\nhelloC super+w\n";
        let actions = parse_sequence(bytes).unwrap();
        assert_eq!(
            actions,
            vec![
                Action::Chord("ctrl+f".into()),
                Action::Text(b"hello".to_vec()),
                Action::Chord("super+w".into()),
            ]
        );
    }

    #[test]
    fn text_may_contain_newlines() {
        let bytes = b"T 5\na\nb\nc";
        let actions = parse_sequence(bytes).unwrap();
        assert_eq!(actions, vec![Action::Text(b"a\nb\nc".to_vec())]);
    }
}
