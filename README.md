# Application layer

Omarchy plugin `jff.app-layer`. `Alt+Space` opens a hint for the focused window. The next key runs that entry from `applications.conf`, then the layer closes.

Install:

```
omarchy plugin add https://github.com/johnfogh/oma_hot_keys.git --enable --yes
```

`Service.qml` adds this line to `~/.config/hypr/bindings.lua` when it is missing:

```lua
dofile(os.getenv("HOME") .. "/.config/omarchy/plugins/jff.app-layer/hyprland.lua")
```

Build the hint program after install:

```
cargo build --release --manifest-path ~/.config/omarchy/plugins/jff.app-layer/Cargo.toml
```

## What happens

`Alt+Space` reads `applications.conf` once. It keeps the generic keys and applies the `[class]` section whose name matches the focused window. Other sections are parsed too, but only that window's section overrides the generic keys.

The hint lists the layer key and the label. The application name is at the top. The window stays hidden until it is centered in the bottom quarter of the display, then it appears.

The next key sends every field on that line except the label, in order, and the layer closes. The hint program does the sending: a chord goes to the focused application, a chord that contains `super` is pressed for Hyprland, and a quoted string is typed into the application.

`Escape`, `Alt+Space`, or focusing a different window closes the hint and leaves the layer without sending anything. `` ` `` opens `applications.conf` in the editor.

## Config

`applications.conf` uses one line per hotkey:

```
layer_hotkey, sent, ..., label
```

The last comma-separated field is the label shown in the hint. Every field before it is sent. A chord is `ctrl`, `alt`, `shift`, `super`, and a key joined with `+`. A double-quoted string is typed as text. Escapes in double quotes are `\n`, `\t`, `\r`, `\\`, `\"`, and `\xNN`. Single quotes are literal.

```
f, ctrl+f, find
w, super+w, quit
u, "/usage", "\r", usage
-, ctrl+-, ctrl+-, ctrl+-, zoom out
```

`super+w` runs Hyprland's close-window binding instead of being typed into the application.

Lines before the first `[class]` are the generic keys. A section name is the Hyprland window class. Its lines override the generic key with the same name, and they apply only while that class is focused. A blank label, or the label `DISABLED`, hides the key and sends nothing. `DISABLED` does not produce a warning.

```
s, ctrl+s, save

[foot]
s, DISABLED
w, super+w, quit
```

The layer keys are letters, `space`, `-`, and `=`.

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
