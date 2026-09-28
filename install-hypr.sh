#!/bin/bash
# Ensure Hyprland loads this plugin's bindings. Safe to run more than once.
set -euo pipefail

bindings="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/bindings.lua"
marker='dofile(os.getenv("HOME") .. "/.config/omarchy/plugins/jff.app-layer/hyprland.lua")'

mkdir -p "$(dirname "$bindings")"
touch "$bindings"

if grep -qF 'plugins/jff.app-layer/hyprland.lua' "$bindings"; then
  exit 0
fi

printf '\n-- Application layer bindings. Installed by the jff.app-layer plugin.\n%s\n' "$marker" >> "$bindings"
hyprctl reload >/dev/null 2>&1 || true
