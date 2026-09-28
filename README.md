# Common in-app hotkeys

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

Each line is `layer_hotkey, keystroke_sent, label`. Leave a label blank to
hide that key and do nothing when it is pressed. An application with no
section uses the generic keys.

`` ` `` opens `applications.conf` in the editor.
