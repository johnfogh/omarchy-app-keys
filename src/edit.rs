//! In-app configuration dialog for applications.conf (F1 in the layer).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gdk::{Key, ModifierType};
use gtk::glib::Propagation;
use gtk::prelude::*;
use gtk::{
    gio, glib, Application, ApplicationWindow, Button, CssProvider, Entry, EventControllerKey,
    Grid, Label, Orientation, Box as GtkBox, Widget,
};

use crate::conf::{self, CommandLine, ConfFile};
use crate::{system_font, theme_colors};

const APP_ID: &str = "jff.AppLayerConfig";
const COL_KEY: i32 = 0;
const COL_SENT: i32 = 1;
const COL_LABEL: i32 = 2;
const COL_GLOBAL: i32 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Generic,
    Class,
}

struct RowWidgets {
    id: usize,
    /// When true, row is a global hotkey (`x`); application rows use a space.
    is_global: bool,
    key: Entry,
    sent: Entry,
    label: Entry,
    global: Entry,
}

impl RowWidgets {
    fn kind(&self) -> RowKind {
        if self.is_global {
            RowKind::Generic
        } else {
            RowKind::Class
        }
    }
}

struct EditorState {
    path: PathBuf,
    class: String,
    conf: ConfFile,
    rows: Vec<RowWidgets>,
    grid: Grid,
    next_grid_row: i32,
    next_row_id: usize,
    suppress_toggle: bool,
    /// Index into `rows` for Up/Down row selection.
    selected: Option<usize>,
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

fn window_title(class: &str) -> String {
    if class.is_empty() {
        "Configuration".into()
    } else {
        format!("Configuration {class}")
    }
}

fn build_dialog(app: &Application, path: PathBuf, class: String, conf: ConfFile) {
    let colors = theme_colors();
    let font = system_font();
    let css = format!(
        "window {{ background-color: {bg}; color: {fg}; font-family: \"{font}\"; font-size: 11pt; }}
         entry {{ background-color: {bg}; color: {fg}; caret-color: {accent}; min-width: 6em; }}
         entry.generic {{ color: {fg}; opacity: 0.75; }}
         entry.class-row {{ color: {accent}; }}
         label.heading {{ color: {accent}; font-weight: 700; }}
         label.column {{ color: {fg}; font-weight: 700; }}
         label.hint {{ color: {fg}; opacity: 0.7; }}
         button {{ padding: 4px 10px; }}
         entry.global-mark {{
           min-width: 1.5em;
           max-width: 2em;
           font-family: monospace;
         }}
         entry.row-selected {{
           background-color: {sel_bg};
           color: {sel_fg};
           caret-color: {sel_fg};
         }}",
        bg = colors.background,
        fg = colors.foreground,
        accent = colors.accent,
        sel_bg = colors.selection_background,
        sel_fg = colors.selection_foreground,
    );
    let provider = CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    }

    let title = window_title(&class);
    let window = ApplicationWindow::builder()
        .application(app)
        .title(&title)
        .resizable(true)
        .default_width(780)
        .default_height(480)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 12);
    root.set_margin_top(16);
    root.set_margin_bottom(16);

    let columns = GtkBox::new(Orientation::Horizontal, 0);
    columns.set_hexpand(true);
    columns.set_vexpand(true);
    let left_gutter = GtkBox::new(Orientation::Vertical, 0);
    let right_gutter = GtkBox::new(Orientation::Vertical, 0);
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_hexpand(true);
    content.set_vexpand(true);
    columns.append(&left_gutter);
    columns.append(&content);
    columns.append(&right_gutter);

    let heading = Label::new(Some(&title));
    heading.set_xalign(0.0);
    heading.add_css_class("heading");
    content.append(&heading);

    let grid = Grid::builder()
        .column_spacing(12)
        .row_spacing(6)
        .column_homogeneous(false)
        .build();
    grid.set_hexpand(true);
    grid.set_vexpand(true);
    grid.set_halign(gtk::Align::Fill);
    grid.set_valign(gtk::Align::Fill);

    for (column, name) in [
        (COL_KEY, "Key"),
        (COL_SENT, "Sent"),
        (COL_LABEL, "Label"),
        (COL_GLOBAL, "Global"),
    ] {
        let label = Label::new(Some(name));
        label.set_xalign(0.0);
        label.add_css_class("column");
        if column == COL_SENT {
            label.set_hexpand(true);
        }
        grid.attach(&label, column, 0, 1, 1);
    }

    let state = Rc::new(RefCell::new(EditorState {
        path,
        class: class.clone(),
        conf,
        rows: Vec::new(),
        grid: grid.clone(),
        next_grid_row: 1,
        next_row_id: 1,
        suppress_toggle: false,
        selected: Some(0),
    }));

    {
        let mut editor = state.borrow_mut();
        let generic = editor.conf.generic.clone();
        for command in generic {
            append_row(&mut editor, &state, RowKind::Generic, Some(&command));
        }
        if !editor.class.is_empty() {
            let commands = editor
                .conf
                .section(&editor.class)
                .map(|section| section.commands.clone())
                .unwrap_or_default();
            for command in commands {
                append_row(&mut editor, &state, RowKind::Class, Some(&command));
            }
            append_row(&mut editor, &state, RowKind::Class, None);
        } else {
            append_row(&mut editor, &state, RowKind::Generic, None);
        }
    }

    let scroll = gtk::ScrolledWindow::builder()
        .child(&grid)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .propagate_natural_height(true)
        .build();
    scroll.set_hexpand(true);
    scroll.set_vexpand(true);
    let table_wrap = GtkBox::new(Orientation::Vertical, 0);
    table_wrap.set_hexpand(true);
    table_wrap.set_vexpand(true);
    table_wrap.append(&scroll);
    content.append(&table_wrap);
    {
        let table_wrap = table_wrap.clone();
        table_wrap.connect_map(move |table_wrap| {
            apply_char_margin(table_wrap, 2);
        });
    }
    apply_char_margin(&table_wrap, 2);

    let footer = GtkBox::new(Orientation::Horizontal, 16);
    footer.set_hexpand(true);

    let hints = Label::new(Some("Ctrl+X delete row"));
    hints.set_xalign(0.0);
    hints.set_hexpand(true);
    hints.add_css_class("hint");
    footer.append(&hints);

    let cancel = Button::with_label("Cancel  Esc");
    cancel.set_tooltip_text(Some("Close without saving (Esc)"));
    let submit = Button::with_label("Submit  Ctrl+S");
    submit.set_tooltip_text(Some("Save applications.conf (Ctrl+S)"));
    footer.append(&cancel);
    footer.append(&submit);
    content.append(&footer);

    root.append(&columns);
    window.set_child(Some(&root));

    {
        let left_gutter = left_gutter.clone();
        let right_gutter = right_gutter.clone();
        columns.connect_notify_local(Some("width"), move |columns, _| {
            sync_side_gutters(columns, &left_gutter, &right_gutter);
        });
    }
    {
        let left_gutter = left_gutter;
        let right_gutter = right_gutter;
        columns.connect_map(move |columns| {
            sync_side_gutters(columns, &left_gutter, &right_gutter);
        });
    }

    let cancel_action = {
        let window = window.clone();
        Rc::new(move || {
            window.close();
        })
    };
    let submit_action = {
        let window = window.clone();
        let state = Rc::clone(&state);
        Rc::new(move || {
            if let Err(err) = save_from_state(&state.borrow()) {
                eprintln!("save failed: {err}");
                return;
            }
            window.close();
        })
    };

    {
        let cancel_action = Rc::clone(&cancel_action);
        cancel.connect_clicked(move |_| cancel_action());
    }
    {
        let submit_action = Rc::clone(&submit_action);
        submit.connect_clicked(move |_| submit_action());
    }

    let controller = EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let state = Rc::clone(&state);
        let cancel_action = Rc::clone(&cancel_action);
        let submit_action = Rc::clone(&submit_action);
        controller.connect_key_pressed(move |_, key, _, mods| {
            let ctrl = mods.contains(ModifierType::CONTROL_MASK);
            if key == Key::Escape {
                cancel_action();
                return Propagation::Stop;
            }
            if ctrl && matches!(key, Key::s | Key::S) {
                submit_action();
                return Propagation::Stop;
            }
            if ctrl && matches!(key, Key::x | Key::X) {
                // Delete the selected row; never close the dialog.
                delete_selected_row(&state);
                return Propagation::Stop;
            }
            match key {
                Key::Up | Key::Down => {
                    select_row_by_arrow(&state, key);
                    Propagation::Stop
                }
                Key::Left | Key::Right => {
                    if move_focus_in_selected_row(&state, key) {
                        Propagation::Stop
                    } else {
                        Propagation::Proceed
                    }
                }
                _ => Propagation::Proceed,
            }
        });
    }
    window.add_controller(controller);
    {
        let state = Rc::clone(&state);
        window.connect_map(move |_| {
            let mut editor = state.borrow_mut();
            sync_all_global_checkboxes(&mut editor);
            apply_selection(&mut editor);
        });
    }
    window.present();
    {
        let mut editor = state.borrow_mut();
        sync_all_global_checkboxes(&mut editor);
        apply_selection(&mut editor);
    }
}

fn side_gutter(width: i32) -> i32 {
    (width * 2) / 16
}

fn char_margin_px(widget: &impl IsA<Widget>, count: i32) -> i32 {
    let layout = widget.create_pango_layout(Some("0"));
    let (width, _) = layout.pixel_size();
    let cell = if width > 0 { width } else { 7 };
    cell * count
}

fn apply_char_margin(widget: &impl IsA<Widget>, count: i32) {
    let margin = char_margin_px(widget, count);
    widget.set_margin_top(margin);
    widget.set_margin_bottom(margin);
    widget.set_margin_start(margin);
    widget.set_margin_end(margin);
}

fn sync_side_gutters(host: &impl IsA<Widget>, left: &GtkBox, right: &GtkBox) {
    let width = host.width();
    if width <= 0 {
        return;
    }
    let margin = side_gutter(width);
    left.set_size_request(margin, -1);
    right.set_size_request(margin, -1);
}

fn style_row(editor: &mut EditorState, index: usize) {
    let class_available = !editor.class.is_empty();
    let is_global = editor.rows[index].is_global;
    let css = if is_global {
        "generic"
    } else {
        "class-row"
    };
    for entry in [
        editor.rows[index].key.clone(),
        editor.rows[index].sent.clone(),
        editor.rows[index].label.clone(),
        editor.rows[index].global.clone(),
    ] {
        entry.remove_css_class("generic");
        entry.remove_css_class("class-row");
        entry.add_css_class(css);
    }
    // Global rows show "x"; application rows show a space.
    editor.rows[index].global.set_editable(class_available);
    set_global_mark(editor, index);
}

fn global_mark(is_global: bool) -> &'static str {
    if is_global {
        "x"
    } else {
        " "
    }
}

fn set_global_mark(editor: &mut EditorState, index: usize) {
    let mark = global_mark(editor.rows[index].is_global);
    let current = editor.rows[index].global.text();
    if current.as_str() != mark {
        editor.suppress_toggle = true;
        editor.rows[index].global.set_text(mark);
        editor.suppress_toggle = false;
    }
}

fn sync_all_global_checkboxes(editor: &mut EditorState) {
    for index in 0..editor.rows.len() {
        style_row(editor, index);
    }
}

fn apply_selection(editor: &mut EditorState) {
    if editor.rows.is_empty() {
        editor.selected = None;
        return;
    }
    if let Some(selected) = editor.selected {
        if selected >= editor.rows.len() {
            editor.selected = Some(editor.rows.len() - 1);
        }
    } else {
        editor.selected = Some(0);
    }
    let selected = editor.selected;
    for (index, row) in editor.rows.iter().enumerate() {
        let on = selected == Some(index);
        for entry in [&row.key, &row.sent, &row.label, &row.global] {
            if on {
                entry.add_css_class("row-selected");
            } else {
                entry.remove_css_class("row-selected");
            }
        }
    }
}

fn select_row_by_arrow(state: &Rc<RefCell<EditorState>>, key: Key) {
    let mut editor = state.borrow_mut();
    if editor.rows.is_empty() {
        return;
    }
    // Prefer moving from the focused row when focus and selection disagree.
    let current = focused_row_index(&editor).or(editor.selected).unwrap_or(0);
    let next = match key {
        Key::Up => current.saturating_sub(1),
        Key::Down => (current + 1).min(editor.rows.len() - 1),
        _ => return,
    };
    editor.selected = Some(next);
    apply_selection(&mut editor);
    editor.rows[next].key.grab_focus();
}

fn move_focus_in_selected_row(state: &Rc<RefCell<EditorState>>, key: Key) -> bool {
    let editor = state.borrow();
    let Some(row_index) = editor.selected.or_else(|| focused_row_index(&editor)) else {
        return false;
    };
    let row = &editor.rows[row_index];
    let cells = [&row.key, &row.sent, &row.label, &row.global];
    let focused = cells.iter().position(|entry| entry.has_focus()).unwrap_or(0);
    let next = match key {
        Key::Left => focused.saturating_sub(1),
        Key::Right => (focused + 1).min(cells.len() - 1),
        _ => return false,
    };
    if next == focused {
        return false;
    }
    cells[next].grab_focus();
    true
}

fn detach_row(grid: &Grid, row: &RowWidgets) {
    grid.remove(&row.key);
    grid.remove(&row.sent);
    grid.remove(&row.label);
    grid.remove(&row.global);
}

fn attach_row_at(grid: &Grid, row: &RowWidgets, row_index: i32) {
    grid.attach(&row.key, COL_KEY, row_index, 1, 1);
    grid.attach(&row.sent, COL_SENT, row_index, 1, 1);
    grid.attach(&row.label, COL_LABEL, row_index, 1, 1);
    grid.attach(&row.global, COL_GLOBAL, row_index, 1, 1);
}

fn relayout(editor: &mut EditorState) {
    let selected_id = editor
        .selected
        .and_then(|index| editor.rows.get(index).map(|row| row.id));
    editor.suppress_toggle = true;
    for row in &editor.rows {
        detach_row(&editor.grid, row);
    }
    // Keep global defaults first, application rows after.
    let mut defaults = Vec::new();
    let mut apps = Vec::new();
    for row in editor.rows.drain(..) {
        if row.is_global {
            defaults.push(row);
        } else {
            apps.push(row);
        }
    }
    editor.rows.extend(defaults);
    editor.rows.extend(apps);

    editor.next_grid_row = 1;
    for index in 0..editor.rows.len() {
        let row_index = editor.next_grid_row;
        editor.next_grid_row += 1;
        attach_row_at(&editor.grid, &editor.rows[index], row_index);
        style_row(editor, index);
    }
    editor.selected = selected_id.and_then(|id| editor.rows.iter().position(|row| row.id == id));
    if editor.selected.is_none() && !editor.rows.is_empty() {
        editor.selected = Some(0);
    }
    apply_selection(editor);
    editor.suppress_toggle = false;
}

fn append_row(
    editor: &mut EditorState,
    state: &Rc<RefCell<EditorState>>,
    kind: RowKind,
    command: Option<&CommandLine>,
) {
    let id = editor.next_row_id;
    editor.next_row_id += 1;

    let is_global = kind == RowKind::Generic;
    let key = Entry::new();
    let sent = Entry::new();
    let label = Entry::new();
    let global = Entry::new();
    global.add_css_class("global-mark");
    global.set_max_length(1);
    global.set_width_chars(1);
    global.set_max_width_chars(1);
    gtk::prelude::EntryExt::set_alignment(&global, 0.5);
    global.set_text(global_mark(is_global));
    global.set_tooltip_text(Some("Press space or x to toggle global / application"));
    // With no class open, every row is global; mark stays "x" and locked.
    global.set_editable(!editor.class.is_empty());
    sent.set_hexpand(true);
    if let Some(command) = command {
        key.set_text(&command.key);
        sent.set_text(&conf::format_actions(&command.actions));
        label.set_text(&command.label);
    }

    let row = RowWidgets {
        id,
        is_global,
        key: key.clone(),
        sent: sent.clone(),
        label: label.clone(),
        global: global.clone(),
    };

    let row_index = editor.next_grid_row;
    editor.next_grid_row += 1;
    attach_row_at(&editor.grid, &row, row_index);
    editor.rows.push(row);
    let index = editor.rows.len() - 1;
    style_row(editor, index);

    for entry in [key, sent, label] {
        let state = Rc::clone(state);
        entry.connect_changed(move |_| {
            ensure_trailing_blank(&state);
        });
    }

    {
        let state = Rc::clone(state);
        let controller = EventControllerKey::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        controller.connect_key_pressed(move |_, key, _, _| {
            if matches!(key, Key::space | Key::x | Key::X) {
                // Defer past this GTK trampoline. Updating Entry text while a
                // borrow_mut is held re-enters `changed` and used to SIGABRT
                // with `RefCell already mutably borrowed`.
                let state = Rc::clone(&state);
                glib::idle_add_local_once(move || {
                    toggle_global_for_id(&state, id);
                });
                Propagation::Stop
            } else if matches!(
                key,
                Key::Up
                    | Key::Down
                    | Key::Left
                    | Key::Right
                    | Key::Tab
                    | Key::ISO_Left_Tab
                    | Key::Escape
            ) {
                Propagation::Proceed
            } else {
                // Ignore other characters in the one-char Global field.
                Propagation::Stop
            }
        });
        global.add_controller(controller);
    }
    {
        let state = Rc::clone(state);
        global.connect_changed(move |_| {
            // Never hard-borrow here: set_text from style_row/relayout can fire
            // while an outer borrow_mut is still active.
            let Ok(editor) = state.try_borrow() else {
                return;
            };
            if editor.suppress_toggle {
                return;
            }
            drop(editor);
            let mut editor = match state.try_borrow_mut() {
                Ok(editor) => editor,
                Err(_) => return,
            };
            let Some(index) = find_row_index(&editor, id) else {
                return;
            };
            set_global_mark(&mut editor, index);
        });
    }
}

fn find_row_index(editor: &EditorState, id: usize) -> Option<usize> {
    editor.rows.iter().position(|row| row.id == id)
}

fn toggle_global_for_id(state: &Rc<RefCell<EditorState>>, id: usize) {
    // try_borrow: style_row may update text while relayout/delete already holds the borrow.
    let mut editor = match state.try_borrow_mut() {
        Ok(editor) => editor,
        Err(_) => return,
    };
    if editor.suppress_toggle {
        return;
    }
    let class_available = !editor.class.is_empty();
    let Some(index) = find_row_index(&editor, id) else {
        return;
    };
    if !class_available {
        editor.rows[index].is_global = true;
        style_row(&mut editor, index);
        return;
    }
    editor.rows[index].is_global = !editor.rows[index].is_global;
    if !row_filled(&editor.rows[index]) {
        style_row(&mut editor, index);
        return;
    }
    relayout(&mut editor);
    drop(editor);
    ensure_trailing_blank(state);
}

fn delete_selected_row(state: &Rc<RefCell<EditorState>>) {
    let id = {
        let editor = state.borrow();
        let index = editor.selected.or_else(|| focused_row_index(&editor));
        index.and_then(|index| editor.rows.get(index).map(|row| row.id))
    };
    let Some(id) = id else {
        return;
    };
    delete_row_by_id(state, id);
}

fn delete_row_by_id(state: &Rc<RefCell<EditorState>>, id: usize) {
    {
        let mut editor = state.borrow_mut();
        let Some(index) = find_row_index(&editor, id) else {
            return;
        };
        let row = editor.rows.remove(index);
        detach_row(&editor.grid, &row);
        if editor.rows.is_empty() {
            editor.selected = None;
        } else {
            editor.selected = Some(index.min(editor.rows.len() - 1));
        }
        relayout(&mut editor);
        if let Some(selected) = editor.selected {
            editor.rows[selected].key.grab_focus();
        }
    }
    ensure_trailing_blank(state);
    apply_selection(&mut state.borrow_mut());
}

fn focused_row_index(editor: &EditorState) -> Option<usize> {
    editor.rows.iter().position(|row| {
        row.key.has_focus()
            || row.sent.has_focus()
            || row.label.has_focus()
            || row.global.has_focus()
    })
}

fn ensure_trailing_blank(state: &Rc<RefCell<EditorState>>) {
    let needs = {
        let editor = state.borrow();
        match editor.rows.last() {
            Some(row) => row_filled(row),
            None => true,
        }
    };
    if needs {
        let mut editor = state.borrow_mut();
        let kind = if editor.class.is_empty() {
            RowKind::Generic
        } else {
            RowKind::Class
        };
        append_row(&mut editor, state, kind, None);
    }
}

fn row_filled(row: &RowWidgets) -> bool {
    !row.key.text().trim().is_empty()
        || !row.sent.text().trim().is_empty()
        || !row.label.text().trim().is_empty()
}

fn collect_commands(rows: &[RowWidgets], kind: RowKind) -> Result<Vec<CommandLine>, String> {
    let mut commands = Vec::new();
    for row in rows {
        if row.kind() != kind || !row_filled(row) {
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
    conf.generic = collect_commands(&editor.rows, RowKind::Generic)?;
    if !editor.class.is_empty() {
        let commands = collect_commands(&editor.rows, RowKind::Class)?;
        conf.section_mut(&editor.class).commands = commands;
    }
    conf::save(&editor.path, &conf)
}

#[cfg(test)]
mod tests {
    use super::{side_gutter, window_title};

    #[test]
    fn side_gutters_are_two_sixteenths() {
        assert_eq!(side_gutter(800), 100);
        assert_eq!(side_gutter(16), 2);
        assert_eq!(side_gutter(720), 90);
    }

    #[test]
    fn configuration_title_includes_application() {
        assert_eq!(window_title(""), "Configuration");
        assert_eq!(window_title("foot"), "Configuration foot");
        assert_eq!(
            window_title("brave-browser"),
            "Configuration brave-browser"
        );
    }
}
