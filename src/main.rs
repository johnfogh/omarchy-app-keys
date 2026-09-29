mod chord;
mod conf;
mod edit;
mod send;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gio, glib, Application, ApplicationWindow, Box, CssProvider, Grid, Label, Orientation};

const COLUMNS: i32 = 3;
const APP_ID: &str = "jff.AppLayerHint";

pub(crate) struct Colors {
    pub background: String,
    pub foreground: String,
    pub accent: String,
}

pub(crate) fn theme_colors() -> Colors {
    let mut colors = Colors {
        background: "#1B1B1B".into(),
        foreground: "#efebdc".into(),
        accent: "#e75a50".into(),
    };
    let path = dirs_home().join(".local/state/omarchy/current/theme/colors.toml");
    let Ok(text) = fs::read_to_string(path) else {
        return colors;
    };
    for (name, slot) in [
        ("background", &mut colors.background),
        ("foreground", &mut colors.foreground),
        ("accent", &mut colors.accent),
    ] {
        if let Some(value) = toml_hex(&text, name) {
            *slot = value;
        }
    }
    colors
}

fn toml_hex(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let rest = line.strip_prefix(key)?.trim_start();
        let rest = rest.strip_prefix('=')?.trim();
        let rest = rest.strip_prefix('"')?;
        let hex = rest.strip_suffix('"')?;
        if hex.len() == 7 && hex.starts_with('#') && hex[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(hex.to_string());
        }
    }
    None
}

pub(crate) fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default()
}

pub(crate) fn system_font() -> String {
    if let Ok(output) = Command::new("omarchy").args(["font", "current"]).output() {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() && !name.eq_ignore_ascii_case("unknown") {
                return name.replace('"', "");
            }
        }
    }
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "font-name"])
        .output()
    {
        if output.status.success() {
            let raw = String::from_utf8_lossy(&output.stdout).trim().trim_matches('\'').to_string();
            if let Some((name, _)) = raw.rsplit_once(' ') {
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    "JetBrainsMono Nerd Font".into()
}

fn parse_hint(text: &str) -> (String, Vec<String>, Vec<(String, String)>) {
    let mut application = String::new();
    let mut warnings = Vec::new();
    let mut options = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("# ") {
            application = name.trim().to_string();
            continue;
        }
        if let Some(warning) = line.strip_prefix("! ") {
            let warning = warning.trim();
            if !warning.is_empty() {
                warnings.push(warning.to_string());
            }
            continue;
        }
        let mut parts = line.split('\t');
        let Some(key) = parts.next().filter(|key| !key.is_empty()) else {
            continue;
        };
        let Some(label) = parts.next() else {
            continue;
        };
        options.push((key.to_string(), label.to_string()));
    }
    if options.is_empty() {
        options = ["space palette", "f find", "n new", "w close", "s save", "j next", "k previous"]
            .into_iter()
            .map(|row| {
                let (key, label) = row.split_once(' ').unwrap();
                (key.to_string(), label.to_string())
            })
            .collect();
    }
    if application.is_empty() {
        application = "generic".into();
    }
    (application, warnings, options)
}

fn load_options(path: &Path) -> (String, Vec<String>, Vec<(String, String)>) {
    match fs::read_to_string(path) {
        Ok(text) => parse_hint(&text),
        Err(_) => parse_hint(""),
    }
}

fn replace_previous(pidfile: &Path) {
    let Ok(text) = fs::read_to_string(pidfile) else {
        return;
    };
    let Ok(pid) = text.trim().parse::<i32>() else {
        return;
    };
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
}

fn remove_own_pidfile(pidfile: &Path) {
    let Ok(text) = fs::read_to_string(pidfile) else {
        return;
    };
    if text.trim().parse::<i32>().ok() == Some(std::process::id() as i32) {
        let _ = fs::remove_file(pidfile);
    }
}

fn build_ui(app: &Application, spec: &Path) {
    let colors = theme_colors();
    let font = system_font();
    let css = format!(
        "window {{ background-color: {bg}; color: {fg}; font-family: \"{font}\"; font-size: 11pt; }}
         .hotkey {{ color: {accent}; font-weight: 700; }}
         .command {{ color: {fg}; }}
         .application {{ color: {accent}; font-weight: 700; }}
         .warning {{ color: {accent}; }}",
        bg = colors.background,
        fg = colors.foreground,
        accent = colors.accent,
    );
    let provider = CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Application layer")
        .decorated(false)
        .resizable(false)
        .build();

    let (application, warnings, options) = load_options(spec);
    let column = Box::new(Orientation::Vertical, 10);
    column.set_margin_top(16);
    column.set_margin_bottom(16);
    column.set_margin_start(20);
    column.set_margin_end(20);

    let heading = Label::new(Some(&application));
    heading.set_xalign(0.0);
    heading.add_css_class("application");
    column.append(&heading);

    for warning in &warnings {
        let label = Label::new(Some(&format!("warning: {warning}")));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.add_css_class("warning");
        column.append(&label);
    }

    let grid = Grid::builder().column_spacing(28).row_spacing(8).build();
    for (index, (key, label)) in options.iter().enumerate() {
        let row = (index as i32) / COLUMNS;
        let column_index = (index as i32) % COLUMNS;
        let cell = Box::new(Orientation::Horizontal, 10);
        let hotkey = Label::new(Some(key));
        hotkey.set_xalign(0.0);
        hotkey.add_css_class("hotkey");
        let command = Label::new(Some(label));
        command.set_xalign(0.0);
        command.add_css_class("command");
        cell.append(&hotkey);
        cell.append(&command);
        grid.attach(&cell, column_index, row, 1, 1);
    }
    column.append(&grid);
    window.set_child(Some(&column));
    window.present();
    thread::spawn(place_in_bottom_quarter);
}

fn set_hint_opacity(address: &str, opacity: i32) {
    let dispatch = format!(
        "hl.dsp.window.set_prop({{ window = \"address:{address}\", prop = \"opacity\", value = {opacity} }})"
    );
    let _ = Command::new("hyprctl").args(["dispatch", &dispatch]).status();
}

fn place_in_bottom_quarter() {
    let mut hidden = false;
    for _ in 0..30 {
        let Some(address) = hint_address() else {
            thread::sleep(Duration::from_millis(20));
            continue;
        };
        if !hidden {
            set_hint_opacity(&address, 0);
            let float = format!(
                "hl.dsp.window.float({{ window = \"address:{address}\", action = \"set\" }})"
            );
            let _ = Command::new("hyprctl").args(["dispatch", &float]).status();
            hidden = true;
        }
        if let Some((address, x, y)) = bottom_quarter_target() {
            let dispatch = format!(
                "hl.dsp.window.move({{ window = \"address:{address}\", x = {x}, y = {y} }})"
            );
            if Command::new("hyprctl")
                .args(["dispatch", &dispatch])
                .status()
                .is_ok_and(|status| status.success())
            {
                set_hint_opacity(&address, 1);
                return;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    if let Some(address) = hint_address() {
        set_hint_opacity(&address, 1);
    }
}

fn hint_address() -> Option<String> {
    let clients = hypr_json("clients")?;
    let hint = clients.iter().find(|client| client["title"] == "Application layer")?;
    Some(hint["address"].as_str()?.to_string())
}

/// Hyprland window coordinates are the monitor size divided by its scale.
fn layout_size(width: i32, height: i32, scale: f64) -> (i32, i32) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    ((width as f64 / scale) as i32, (height as f64 / scale) as i32)
}

/// Center of the display's bottom quarter, clamped so the hint stays on screen.
fn hint_origin(origin_x: i32, origin_y: i32, width: i32, height: i32, hint_w: i32, hint_h: i32) -> (i32, i32) {
    let max_x = origin_x + (width - hint_w).max(0);
    let max_y = origin_y + (height - hint_h).max(0);
    let x = (origin_x + (width - hint_w) / 2).clamp(origin_x, max_x);
    let y = (origin_y + (height * 7 / 8) - (hint_h / 2)).clamp(origin_y, max_y);
    (x, y)
}

fn bottom_quarter_target() -> Option<(String, i32, i32)> {
    let clients = hypr_json("clients")?;
    let monitors = hypr_json("monitors")?;
    let hint = clients.iter().find(|client| client["title"] == "Application layer")?;
    let hint_size = hint.get("size")?.as_array()?;
    let hint_w = hint_size.first()?.as_i64()? as i32;
    let hint_h = hint_size.get(1)?.as_i64()? as i32;
    if hint_w <= 0 || hint_h <= 0 {
        return None;
    }
    let monitor_id = hint.get("monitor").and_then(|value| value.as_i64());
    let monitor = monitors
        .iter()
        .find(|monitor| monitor_id.is_some_and(|id| monitor["id"].as_i64() == Some(id)))
        .or_else(|| monitors.iter().find(|monitor| monitor["focused"] == true))?;
    let scale = monitor["scale"].as_f64().unwrap_or(1.0);
    let (width, height) = layout_size(
        monitor["width"].as_i64()? as i32,
        monitor["height"].as_i64()? as i32,
        scale,
    );
    let (x, y) = hint_origin(
        monitor["x"].as_i64()? as i32,
        monitor["y"].as_i64()? as i32,
        width,
        height,
        hint_w,
        hint_h,
    );
    Some((hint["address"].as_str()?.to_string(), x, y))
}

fn hypr_json(kind: &str) -> Option<Vec<serde_json::Value>> {
    let output = Command::new("hyprctl").args([kind, "-j"]).output().ok()?;
    serde_json::from_slice(&output.stdout).ok()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(first) = args.next() else {
        eprintln!("usage: app-layer-hint <pidfile> <spec>");
        eprintln!("       app-layer-hint chord <mod+key>");
        eprintln!("       app-layer-hint send <window> <sequence>");
        eprintln!("       app-layer-hint edit <applications.conf> <class>");
        std::process::exit(2);
    };
    if first == "edit" {
        let Some(path) = args.next() else {
            eprintln!("usage: app-layer-hint edit <applications.conf> <class>");
            std::process::exit(2);
        };
        let class = args.next().unwrap_or_default();
        if let Err(err) = edit::run(std::path::Path::new(&path), &class) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }
    if first == "send" {
        let Some(window) = args.next() else {
            eprintln!("usage: app-layer-hint send <window> <sequence>");
            std::process::exit(2);
        };
        let Some(sequence) = args.next() else {
            eprintln!("usage: app-layer-hint send <window> <sequence>");
            std::process::exit(2);
        };
        if let Err(err) = send::send_sequence(&window, std::path::Path::new(&sequence)) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }
    if first == "chord" {
        let Some(chord) = args.next() else {
            eprintln!("usage: app-layer-hint chord <mod+key>");
            std::process::exit(2);
        };
        if let Err(err) = chord::press_chord(&chord) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }
    let pidfile = std::path::PathBuf::from(first);
    let spec = args.next().map(std::path::PathBuf::from).unwrap_or_default();

    replace_previous(&pidfile);
    let _ = fs::write(&pidfile, std::process::id().to_string());

    let app = Application::new(Some(APP_ID), gio::ApplicationFlags::empty());
    let spec_for_ui = spec.clone();
    app.connect_activate(move |app| {
        if app.active_window().is_some() {
            return;
        }
        build_ui(app, &spec_for_ui);
    });

    let pidfile_for_signal = pidfile.clone();
    glib::unix_signal_add(libc::SIGTERM, move || {
        if let Some(app) = gio::Application::default() {
            app.quit();
        }
        remove_own_pidfile(&pidfile_for_signal);
        glib::ControlFlow::Break
    });

    let status = app.run_with_args::<&str>(&[]);
    remove_own_pidfile(&pidfile);
    std::process::exit(status.into());
}

#[cfg(test)]
mod tests {
    use super::{parse_hint, toml_hex};

    #[test]
    fn hint_shows_application_warnings_and_keys() {
        let (application, warnings, options) = parse_hint(
            "# brave-browser\n! n and d both send ctrl+j\n! \nf\tfind\nn\tnew tab\nf1\tconfigure\n",
        );
        assert_eq!(application, "brave-browser");
        assert_eq!(warnings, vec!["n and d both send ctrl+j".to_string()]);
        assert_eq!(
            options,
            vec![
                ("f".to_string(), "find".to_string()),
                ("n".to_string(), "new tab".to_string()),
                ("f1".to_string(), "configure".to_string()),
            ]
        );
    }

    #[test]
    fn empty_hint_uses_generic_commands() {
        let (application, warnings, options) = parse_hint("");
        assert_eq!(application, "generic");
        assert!(warnings.is_empty());
        assert_eq!(options[0], ("space".to_string(), "palette".to_string()));
        assert_eq!(options.len(), 7);
    }

    #[test]
    fn hint_sits_in_the_bottom_quarter() {
        let (x, y) = super::hint_origin(0, 0, 3840, 2160, 400, 200);
        assert_eq!(x, 1720);
        assert_eq!(y, 1790);
        let (_, tall) = super::hint_origin(0, 0, 3840, 2160, 400, 2000);
        assert_eq!(tall, 160);
        assert_eq!(super::layout_size(3840, 2160, 1.25), (3072, 1728));
    }

    #[test]
    fn theme_color_reads_quoted_hex() {
        let text = "background = \"#1B1B1B\"\nforeground = \"#efebdc\"\n";
        assert_eq!(toml_hex(text, "background").as_deref(), Some("#1B1B1B"));
        assert_eq!(toml_hex(text, "missing"), None);
    }
}
