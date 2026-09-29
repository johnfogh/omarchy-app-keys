//! Parse and serialize `applications.conf` for the configuration dialog.

use std::fmt::Write as _;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Chord(String),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    pub key: String,
    pub actions: Vec<Action>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub class: String,
    pub commands: Vec<CommandLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfFile {
    pub header: String,
    pub generic: Vec<CommandLine>,
    pub sections: Vec<Section>,
}

impl ConfFile {
    pub fn section_mut(&mut self, class: &str) -> &mut Section {
        if let Some(index) = self.sections.iter().position(|section| section.class == class) {
            return &mut self.sections[index];
        }
        self.sections.push(Section {
            class: class.to_string(),
            commands: Vec::new(),
        });
        self.sections.last_mut().unwrap()
    }

    pub fn section(&self, class: &str) -> Option<&Section> {
        self.sections.iter().find(|section| section.class == class)
    }
}

pub fn load(path: &Path) -> Result<ConfFile, String> {
    let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    Ok(parse(&text))
}

pub fn save(path: &Path, conf: &ConfFile) -> Result<(), String> {
    let text = serialize(conf);
    std::fs::write(path, text).map_err(|err| err.to_string())
}

pub fn parse(text: &str) -> ConfFile {
    let mut header = String::new();
    let mut generic = Vec::new();
    let mut sections = Vec::new();
    let mut current: Option<usize> = None;
    let mut in_header = true;

    for line in text.lines() {
        let row = line.trim();
        if in_header {
            if row.is_empty() || row.starts_with('#') {
                header.push_str(line);
                header.push('\n');
                continue;
            }
            in_header = false;
        }

        if let Some(name) = row.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            let class = name.trim().to_string();
            sections.push(Section {
                class,
                commands: Vec::new(),
            });
            current = Some(sections.len() - 1);
            continue;
        }
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        if let Some(command) = parse_command_line(row) {
            match current {
                Some(index) => sections[index].commands.push(command),
                None => generic.push(command),
            }
        }
    }

    ConfFile {
        header,
        generic,
        sections,
    }
}

pub fn serialize(conf: &ConfFile) -> String {
    let mut out = String::new();
    if !conf.header.is_empty() {
        out.push_str(&conf.header);
        if !conf.header.ends_with('\n') {
            out.push('\n');
        }
        if !out.ends_with("\n\n") && (!conf.generic.is_empty() || !conf.sections.is_empty()) {
            // Keep a single trailing newline after the header block.
        }
    }
    for command in &conf.generic {
        let _ = writeln!(out, "{}", format_command(command));
    }
    for section in &conf.sections {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.ends_with("\n\n") && !out.is_empty() {
            out.push('\n');
        }
        let _ = writeln!(out, "[{}]", section.class);
        for command in &section.commands {
            let _ = writeln!(out, "{}", format_command(command));
        }
    }
    out
}

pub fn format_command(command: &CommandLine) -> String {
    let mut parts = vec![command.key.clone()];
    if command.label == "DISABLED" && command.actions.is_empty() {
        parts.push("DISABLED".into());
        return parts.join(", ");
    }
    for action in &command.actions {
        parts.push(format_action(action));
    }
    parts.push(command.label.clone());
    parts.join(", ")
}

pub fn format_action(action: &Action) -> String {
    match action {
        Action::Chord(shortcut) => shortcut.clone(),
        Action::Text(text) => quote_double(text),
    }
}

pub fn format_actions(actions: &[Action]) -> String {
    actions
        .iter()
        .map(format_action)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_actions_field(text: &str) -> Result<Vec<Action>, String> {
    let fields = split_fields(text.trim());
    let mut actions = Vec::new();
    for field in fields {
        if field.quoted {
            actions.push(Action::Text(field.value));
        } else if field.value.is_empty() {
            continue;
        } else {
            actions.push(Action::Chord(field.value.to_ascii_lowercase()));
        }
    }
    Ok(actions)
}

pub fn parse_command_line(row: &str) -> Option<CommandLine> {
    let fields = split_fields(row);
    if fields.len() < 2 {
        return None;
    }
    let key = normalize_layer_key(&fields[0].value)?;
    let label = fields[fields.len() - 1].value.trim().to_string();
    let mut actions = Vec::new();
    for field in &fields[1..fields.len() - 1] {
        if field.quoted {
            actions.push(Action::Text(field.value.clone()));
        } else {
            actions.push(Action::Chord(field.value.to_ascii_lowercase()));
        }
    }
    Some(CommandLine { key, actions, label })
}

/// Canonical layer key: mods as ctrl/alt/shift, then base. Bare `f1` and `alt+space` are reserved.
pub fn normalize_layer_key(raw: &str) -> Option<String> {
    let key = raw.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }
    let parts: Vec<&str> = key
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }

    let mut mods = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut base = None;
    for (index, part) in parts.iter().enumerate() {
        let part = match *part {
            "control" => "ctrl",
            "minus" => "-",
            "equal" => "=",
            other => other,
        };
        let is_mod = matches!(part, "ctrl" | "alt" | "shift");
        if index + 1 < parts.len() || is_mod {
            if matches!(part, "super" | "super_l") {
                return None;
            }
            if !is_mod || !seen.insert(part) {
                return None;
            }
            mods.push(part);
        } else {
            base = Some(part);
        }
    }
    let base = base?;
    if !is_base_key(base) {
        return None;
    }
    mods.sort_by_key(|mod_name| match *mod_name {
        "ctrl" => 0,
        "alt" => 1,
        "shift" => 2,
        _ => 3,
    });
    if base == "space" && mods == ["alt"] {
        return None;
    }
    if base == "f1" && mods.is_empty() {
        return None;
    }
    if mods.is_empty() {
        Some(base.to_string())
    } else {
        Some(format!("{}+{base}", mods.join("+")))
    }
}

fn is_base_key(base: &str) -> bool {
    if matches!(base, "-" | "=" | "space") {
        return true;
    }
    if base.len() == 1 && base.as_bytes()[0].is_ascii_lowercase() {
        return true;
    }
    if let Some(n) = base.strip_prefix('f').and_then(|rest| rest.parse::<u32>().ok()) {
        return (1..=12).contains(&n);
    }
    false
}

struct Field {
    quoted: bool,
    value: String,
}

fn split_fields(text: &str) -> Vec<Field> {
    let bytes = text.as_bytes();
    let mut fields = Vec::new();
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        while i < n && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }
        let c = bytes[i];
        if c == b'"' || c == b'\'' {
            let quote = c;
            i += 1;
            let start = i;
            while i < n {
                if quote == b'"' && bytes[i] == b'\\' {
                    i = (i + 2).min(n);
                    continue;
                }
                if bytes[i] == quote {
                    break;
                }
                i += 1;
            }
            let raw = &text[start..i.min(n)];
            if i < n {
                i += 1;
            }
            let value = if quote == b'"' {
                unescape_double(raw)
            } else {
                raw.to_string()
            };
            fields.push(Field { quoted: true, value });
            while i < n && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < n && bytes[i] == b',' {
                i += 1;
            }
        } else {
            let start = i;
            while i < n && bytes[i] != b',' {
                i += 1;
            }
            let value = text[start..i].trim().to_string();
            fields.push(Field {
                quoted: false,
                value,
            });
            if i < n && bytes[i] == b',' {
                i += 1;
            }
        }
    }
    fields
}

fn unescape_double(body: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = body.chars().collect();
    let mut j = 0;
    while j < chars.len() {
        if chars[j] == '\\' && j + 1 < chars.len() {
            match chars[j + 1] {
                'n' => {
                    out.push('\n');
                    j += 2;
                }
                't' => {
                    out.push('\t');
                    j += 2;
                }
                'r' => {
                    out.push('\r');
                    j += 2;
                }
                '\\' | '"' => {
                    out.push(chars[j + 1]);
                    j += 2;
                }
                'x' if j + 3 < chars.len() => {
                    let hex: String = chars[j + 2..j + 4].iter().collect();
                    if let Ok(value) = u8::from_str_radix(&hex, 16) {
                        out.push(value as char);
                        j += 4;
                    } else {
                        out.push(chars[j + 1]);
                        j += 2;
                    }
                }
                other => {
                    out.push(other);
                    j += 2;
                }
            }
        } else {
            out.push(chars[j]);
            j += 1;
        }
    }
    out
}

fn quote_double(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_commands() {
        let text = "\
# header\n\
\n\
f, ctrl+f, find\n\
s, ctrl+s, save\n\
\n\
[foot]\n\
w, super+w, quit\n\
s, DISABLED\n\
u, \"/usage\", \"\\r\", usage\n\
";
        let conf = parse(text);
        assert_eq!(conf.generic.len(), 2);
        assert_eq!(conf.generic[0].key, "f");
        let foot = conf.section("foot").unwrap();
        assert_eq!(foot.commands[0].label, "quit");
        assert_eq!(foot.commands[1].label, "DISABLED");
        assert_eq!(
            foot.commands[2].actions,
            vec![Action::Text("/usage".into()), Action::Text("\r".into())]
        );
        let again = parse(&serialize(&conf));
        assert_eq!(again.generic, conf.generic);
        assert_eq!(again.sections, conf.sections);
    }

    #[test]
    fn normalize_rejects_reserved_keys() {
        assert_eq!(normalize_layer_key("f1"), None);
        assert_eq!(normalize_layer_key("alt+space"), None);
        assert_eq!(normalize_layer_key("shift+f1").as_deref(), Some("shift+f1"));
        assert_eq!(normalize_layer_key("alt+ctrl+a").as_deref(), Some("ctrl+alt+a"));
    }

    #[test]
    fn actions_field_parses_mixed() {
        let actions = parse_actions_field("ctrl+f, \"a, b\"").unwrap();
        assert_eq!(
            actions,
            vec![Action::Chord("ctrl+f".into()), Action::Text("a, b".into())]
        );
    }
}
