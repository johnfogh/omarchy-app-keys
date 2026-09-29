//! In-app configuration dialog for applications.conf (F1 in the layer).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gdk::Key;
use gtk::glib::Propagation;
use gtk::prelude::*;
use gtk::{
    gio, glib, Application, ApplicationWindow, Button, CssProvider, Entry, EventControllerKey,
    Grid, Label, Orientation, Box as GtkBox,
};

use crate::conf::{self, CommandLine, ConfFile};
use crate::{system_font, theme_colors};

const APP_ID: &str = "jff.AppLayerConfig";
const COL_KEY: i32 = 0;
const COL_SENT: i32 = 1;
const COL_LABEL: i32 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Generic,
    Class,
}

struct RowWidgets {
    key: Entry,
    sent: Entry,
    label: Entry,
}

struct EditorState {
    path: PathBuf,
    class: String,
    conf: ConfFile,
    generic_rows: Vec<RowWidgets>,
    class_rows: Vec<RowWidgets>,
    grid: Grid,
    next_grid_row: i32,
}

pub fn run(path: &Path, class: &str) -> Result<(), String> {
    let conf = conf::load(path)?;
    let path = path.to_path_buf();
    let class = class.to_string();

    let app = Application::new(Some(APP_ID), gio::ApplicationFlags::empty());
    app.connect_activate(move |app| {
        if app.active_window().is_some() {
            return;
        }
        build_dialog(app, path.clone(), class.clone(), conf.clone());
    });
    let status = app.run_with_args::<&str>(&[]);
    if status == glib::ExitCode::SUCCESS {
        Ok(())
    } else {
        Err(format!("config dialog exited with {status:?}"))
    }
}

fn build_dialog(app: &Application, path: PathBuf, class: String, conf: ConfFile) {
    let colors = theme_colors();
    let font = system_font();
    let css = format!(
        "window {{ background-color: {bg}; color: {fg}; font-family: \"{font}\"; font-size: 11pt; }}
         entry {{ background-color: {bg}; color: {fg}; caret-color: {accent}; min-width: 8em; }}
         entry.generic {{ color: {fg}; opacity: 0.75; }}
         entry.class-row {{ color: {accent}; }}
         label.heading {{ color: {accent}; font-weight: 700; }}
         label.column {{ color: {fg}; font-weight: 700; }}
         button {{ padding: 6px 14px; }}",
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
        .title("Application layer config")
        .resizable(true)
        .default_width(720)
        .default_height(480)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 12);
    root.set_margin_top(16);
    root.set_margin_bottom(16);
    root.set_margin_start(20);
    root.set_margin_end(20);

    let title = if class.is_empty() {
        "Edit defaults".to_string()
    } else {
        format!("Edit defaults and [{class}]")
    };
    let heading = Label::new(Some(&title));
    heading.set_xalign(0.0);
    heading.add_css_class("heading");
    root.append(&heading);

    let grid = Grid::builder()
        .column_spacing(12)
        .row_spacing(6)
        .build();

    for (column, name) in [(COL_KEY, "Key"), (COL_SENT, "Sent"), (COL_LABEL, "Label")] {
        let label = Label::new(Some(name));
        label.set_xalign(0.0);
        label.add_css_class("column");
        grid.attach(&label, column, 0, 1, 1);
    }

    let state = Rc::new(RefCell::new(EditorState {
        path,
        class: class.clone(),
        conf,
        generic_rows: Vec::new(),
        class_rows: Vec::new(),
        grid: grid.clone(),
        next_grid_row: 1,
    }));

    {
        let mut editor = state.borrow_mut();
        let defaults = Label::new(Some("Defaults"));
        defaults.set_xalign(0.0);
        defaults.add_css_class("heading");
        editor.grid.attach(&defaults, 0, editor.next_grid_row, 3, 1);
        editor.next_grid_row += 1;

        let generic = editor.conf.generic.clone();
        for command in generic {
            append_row(&mut editor, &state, RowKind::Generic, Some(&command));
        }
        append_row(&mut editor, &state, RowKind::Generic, None);

        if !editor.class.is_empty() {
            let class_heading = Label::new(Some(&format!("[{}]", editor.class)));
            class_heading.set_xalign(0.0);
            class_heading.add_css_class("heading");
            editor.grid.attach(&class_heading, 0, editor.next_grid_row, 3, 1);
            editor.next_grid_row += 1;

            let commands = editor
                .conf
                .section(&editor.class)
                .map(|section| section.commands.clone())
                .unwrap_or_default();
            for command in commands {
                append_row(&mut editor, &state, RowKind::Class, Some(&command));
            }
            append_row(&mut editor, &state, RowKind::Class, None);
        }
    }

    root.append(&grid);

    let buttons = GtkBox::new(Orientation::Horizontal, 10);
    buttons.set_halign(gtk::Align::End);
    let cancel = Button::with_label("Cancel");
    let submit = Button::with_label("Submit");
    buttons.append(&cancel);
    buttons.append(&submit);
    root.append(&buttons);

    window.set_child(Some(&root));

    {
        let window = window.clone();
        cancel.connect_clicked(move |_| {
            window.close();
        });
    }
    {
        let window = window.clone();
        let state = Rc::clone(&state);
        submit.connect_clicked(move |_| {
            if let Err(err) = save_from_state(&state.borrow()) {
                eprintln!("save failed: {err}");
                return;
            }
            window.close();
        });
    }

    let controller = EventControllerKey::new();
    {
        let state = Rc::clone(&state);
        controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                Key::Up | Key::Down | Key::Left | Key::Right => {
                    if move_focus(&state.borrow(), key) {
                        Propagation::Stop
                    } else {
                        Propagation::Proceed
                    }
                }
                Key::Escape => {
                    if let Some(app) = gio::Application::default() {
                        app.quit();
                    }
                    Propagation::Stop
                }
                _ => Propagation::Proceed,
            }
        });
    }
    window.add_controller(controller);
    window.present();
}

fn append_row(
    editor: &mut EditorState,
    state: &Rc<RefCell<EditorState>>,
    kind: RowKind,
    command: Option<&CommandLine>,
) {
    let row_index = editor.next_grid_row;
    editor.next_grid_row += 1;

    let key = Entry::new();
    let sent = Entry::new();
    let label = Entry::new();
    let css = match kind {
        RowKind::Generic => "generic",
        RowKind::Class => "class-row",
    };
    for entry in [&key, &sent, &label] {
        entry.add_css_class(css);
    }
    if let Some(command) = command {
        key.set_text(&command.key);
        sent.set_text(&conf::format_actions(&command.actions));
        label.set_text(&command.label);
    }

    editor.grid.attach(&key, COL_KEY, row_index, 1, 1);
    editor.grid.attach(&sent, COL_SENT, row_index, 1, 1);
    editor.grid.attach(&label, COL_LABEL, row_index, 1, 1);

    let row = RowWidgets {
        key: key.clone(),
        sent: sent.clone(),
        label: label.clone(),
    };
    match kind {
        RowKind::Generic => editor.generic_rows.push(row),
        RowKind::Class => editor.class_rows.push(row),
    }

    for entry in [key, sent, label] {
        let state = Rc::clone(state);
        let kind = kind;
        entry.connect_changed(move |_| {
            ensure_trailing_blank(&state, kind);
        });
    }
}

fn ensure_trailing_blank(state: &Rc<RefCell<EditorState>>, kind: RowKind) {
    let needs = {
        let editor = state.borrow();
        let rows = match kind {
            RowKind::Generic => &editor.generic_rows,
            RowKind::Class => &editor.class_rows,
        };
        match rows.last() {
            Some(row) => row_filled(row),
            None => true,
        }
    };
    if needs {
        let mut editor = state.borrow_mut();
        append_row(&mut editor, state, kind, None);
    }
}

fn row_filled(row: &RowWidgets) -> bool {
    !row.key.text().trim().is_empty()
        || !row.sent.text().trim().is_empty()
        || !row.label.text().trim().is_empty()
}

fn collect_commands(rows: &[RowWidgets]) -> Result<Vec<CommandLine>, String> {
    let mut commands = Vec::new();
    for row in rows {
        if !row_filled(row) {
            continue;
        }
        let key_text = row.key.text();
        let key = conf::normalize_layer_key(key_text.as_str())
            .ok_or_else(|| format!("invalid layer key: {}", key_text.as_str()))?;
        let label = row.label.text().trim().to_string();
        let actions = if label == "DISABLED" && row.sent.text().trim().is_empty() {
            Vec::new()
        } else {
            conf::parse_actions_field(row.sent.text().as_str())?
        };
        commands.push(CommandLine {
            key,
            actions,
            label,
        });
    }
    Ok(commands)
}

fn save_from_state(editor: &EditorState) -> Result<(), String> {
    let mut conf = editor.conf.clone();
    conf.generic = collect_commands(&editor.generic_rows)?;
    if !editor.class.is_empty() {
        let commands = collect_commands(&editor.class_rows)?;
        conf.section_mut(&editor.class).commands = commands;
    }
    conf::save(&editor.path, &conf)
}

fn all_entries(editor: &EditorState) -> Vec<Entry> {
    let mut entries = Vec::new();
    for row in editor.generic_rows.iter().chain(editor.class_rows.iter()) {
        entries.push(row.key.clone());
        entries.push(row.sent.clone());
        entries.push(row.label.clone());
    }
    entries
}

fn move_focus(editor: &EditorState, key: Key) -> bool {
    let entries = all_entries(editor);
    if entries.is_empty() {
        return false;
    }
    let focused = entries.iter().position(|entry| entry.has_focus());
    let Some(index) = focused else {
        entries[0].grab_focus();
        return true;
    };
    let columns = 3usize;
    let row = index / columns;
    let col = index % columns;
    let rows = entries.len() / columns;
    let (next_row, next_col) = match key {
        Key::Left => (row, col.saturating_sub(1)),
        Key::Right => (row, (col + 1).min(columns - 1)),
        Key::Up => (row.saturating_sub(1), col),
        Key::Down => ((row + 1).min(rows.saturating_sub(1)), col),
        _ => return false,
    };
    let next = next_row * columns + next_col;
    if next == index {
        return false;
    }
    entries[next].grab_focus();
    true
}
