mod chord;

use std::fs;
use std::path::Path;
use std::process::Command;

use gtk::prelude::*;
use gtk::{gio, glib, Application, ApplicationWindow, Box, CssProvider, Grid, Label, Orientation};

const COLUMNS: i32 = 3;
const APP_ID: &str = "org.omarchy.AppLayerHint";

struct Colors {
    background: String,
    foreground: String,
    accent: String,
}

fn theme_colors() -> Colors {
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

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default()
}

fn system_font() -> String {
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

fn load_options(path: &Path) -> (String, Vec<String>, Vec<(String, String)>) {
    let mut application = String::new();
    let mut warnings = Vec::new();
    let mut options = Vec::new();
    if let Ok(text) = fs::read_to_string(path) {
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
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(first) = args.next() else {
        eprintln!("usage: app-layer-hint <pidfile> <spec>");
        eprintln!("       app-layer-hint chord <mod+key>");
        std::process::exit(2);
    };
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
