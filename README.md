# Application layer

Omarchy plugin `omarchy-app-keys`. `Alt+Space` opens a hint for the focused window. The next layer key runs that entry from `applications.conf`, then the layer closes. `F1` opens an in-app configuration dialog.

## Install

```
omarchy plugin add https://github.com/johnfogh/omarchy-app-keys.git --enable --yes
cargo build --release --manifest-path ~/.config/omarchy/plugins/omarchy-app-keys/Cargo.toml
```

`Service.qml` adds this line to `~/.config/hypr/bindings.lua` when it is missing:

```lua
dofile(os.getenv("HOME") .. "/.config/omarchy/plugins/omarchy-app-keys/hyprland.lua")
```

After `omarchy plugin update omarchy-app-keys`, rebuild the release binary, run the plugin’s `install-hypr.sh` if the bindings line is missing, then `hyprctl reload`.

## What happens

`Alt+Space` reads `applications.conf` once. It keeps the generic keys and applies the `[class]` section whose name matches the focused window. Other sections are parsed too, but only that window’s section overlays the generic keys.

The hint lists each layer key and its label, with the window class at the top, plus `f1` / `configure`. The window stays hidden until it is centered in the bottom quarter of the display, then it appears.

The next matching key sends every field on that line except the label, in order, and the layer closes. The hint program does the sending:

- a normal chord goes to the focused application
- a chord that contains `super` is pressed for Hyprland (so `super+w` can close the window)
- a quoted string is typed into the application

`Escape`, `Alt+Space`, an unbound key, or focusing a different window closes the hint and leaves the layer without sending. `F1` leaves the layer and opens the configuration dialog for the defaults and the focused class.

## Configuration dialog

`F1` in the layer runs `app-layer-hint edit <applications.conf> <class>`.

- Three editable columns: **Key**, **Sent**, **Label**
- Defaults first (muted), then the `[class]` section (accent)
- Arrow keys move between cells
- A blank row stays at the end of each section; filling it adds another
- **Submit** writes both the generic defaults and the current class back to `applications.conf`
- **Cancel** or Escape closes without saving

Bare `F1` is reserved for this dialog. Modified keys such as `shift+f1` can still be layer hotkeys.

## Config

`applications.conf` uses one line per hotkey:

```
layer_hotkey, sent, ..., label
```

The last comma-separated field is the label shown in the hint. Every field between the layer key and the label is sent, in order.

### Layer keys

Base keys: `a`–`z`, `space`, `-`, `=`, and `f1`–`f12`.

Optional modifiers on the layer key: `ctrl` (also `control`), `alt`, and `shift`, joined with `+` in any order (stored as `ctrl`, then `alt`, then `shift`). `super` is not allowed on a layer key. Reserved: `Alt+Space` (the layer itself) and bare `F1` (configure).

```
f, ctrl+f, find
shift+f, ctrl+shift+f, find selection
f2, "help", help
ctrl+f1, ctrl+shift+p, palette
```

### Sent actions

- Unquoted field → chord: `ctrl`, `alt`, `shift`, `super`, and a key joined with `+`
- Double-quoted field → typed text; escapes `\n`, `\t`, `\r`, `\\`, `\"`, `\xNN`
- Single-quoted field → literal text

```
w, super+w, quit
u, "/usage", "\r", usage
-, ctrl+-, ctrl+-, ctrl+-, zoom out
```

### Sections and disabling

Lines before the first `[class]` are the generic keys. A section name is the Hyprland window class. Its lines override the generic key with the same name, and they apply only while that class is focused.

A blank label, or the label `DISABLED`, hides the key and sends nothing. `DISABLED` does not produce a warning.

```
s, ctrl+s, save

[foot]
s, DISABLED
w, super+w, quit
```

Loading the file warns when the same layer key is defined twice, or when two keys send the same chord or string. Keys labeled `DISABLED` are left out of those warnings.

## Generic keys

| Key | Sent | Label |
|---|---|---|
| f | Ctrl+F | find |
| n | Ctrl+N | new |
| w | Ctrl+W | close |
| s | Ctrl+S | save |
| j | Ctrl+Tab | next |
| k | Ctrl+Shift+Tab | previous |
| = | Ctrl+=, three times | zoom in |
| - | Ctrl+-, three times | zoom out |

## Tests

```
lua parse_test.lua
cargo test
```
