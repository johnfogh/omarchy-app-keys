# Common in-app hotkeys

This repository is the Omarchy plugin `jff.app-layer`. Install it with:

```
omarchy plugin add https://github.com/johnfogh/oma_hot_keys.git --yes
```

The shell loads `Service.qml`, which adds the Hyprland bindings in `hyprland.lua` to `~/.config/hypr/bindings.lua` if they are missing. The hotkeys are read from `applications.conf` in the plugin directory.

One set of keys runs the same command in whatever application is focused.
These keys do not launch applications. Launch keys stay on Super+Shift.

`Alt+Space` opens the layer. It is bound in `~/.config/hypr/bindings.lua`.
Each command still needs the keystroke that application actually understands.
Each command key sends one shortcut to the window that was focused when the
layer opened. The hint lists the layer key and the standard command name.

## Leader

`Alt+Space` opens the command layer and leaves a list of every command key
on screen until the layer closes. The stock on-screen display keeps a single
truncated line, so this list is a separate window that does not take focus.
The next key runs one command, then the layer closes. `Escape`, `Alt+Space`
again, or any other key leaves the layer without running a command.

## Commands

| Key | Command | Sent to the focused application |
|---|---|---|
| Space | palette | Ctrl+Shift+P |
| F | find | Ctrl+F |
| N | new | Ctrl+N |
| W | close | Ctrl+W |
| S | save | Ctrl+S |
| J | next | Ctrl+Tab |
| K | previous | Ctrl+Shift+Tab |

`Super+W` already closes the Hyprland window. Inside this layer, `W` only
closes the app's current item.

## Config

All hotkeys live in `applications.conf`. Lines before the first `[class]`
are the generic hotkeys. A section replaces those keys for that Hyprland
window class:

```
f, ctrl+f, find

[brave-browser]
n, ctrl+t, new tab
s, ctrl+s,
```

Each line is `layer_hotkey, sent, ..., label`. Every field except the last
is sent in order. The last field is the label. A sent field is a chord,
such as `ctrl+f`, or a quoted string typed into the focused application.
A chord that uses `super` is given to Hyprland instead, so `super+w` runs
Hyprland's own binding. In double quotes, `\n`, `\t`, `\r`, `\\`, `\"`,
and `\xNN` are escapes:

```
w, super+w, close
```

A `[class]` section
is used only while a window of that class is focused. Its keys take
priority over the generic keys, and a blank label hides that key. Loading
the file warns about a repeated layer key and about two keys that send
the same keystroke.

`` ` `` opens `applications.conf` in the editor.
