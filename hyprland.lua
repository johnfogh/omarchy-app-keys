-- Installed with the omarchy-app-keys Omarchy plugin.
-- Application command layer. Alt+Space enters it; the next key runs one
-- command and the layer closes. Escape or any other key leaves it.
-- The stock OSD keeps a single elided line, so the option list is its own
-- no-focus window and stays up until the layer closes.
local app_layer_runtime = os.getenv("XDG_RUNTIME_DIR") or "/tmp"
local app_layer_pid = app_layer_runtime .. "/omarchy-app-layer-hint.pid"
local app_layer_spec = app_layer_runtime .. "/omarchy-app-layer-hint.tsv"
local function app_layer_root()
  local source = debug.getinfo(1, "S").source
  if source:sub(1, 1) == "@" then
    source = source:sub(2)
  end
  local root = source:match("^(.*)/hyprland%.lua$")
  if root then
    return root
  end
  return os.getenv("HOME") .. "/.config/omarchy/plugins/omarchy-app-keys"
end

local app_layer_root_dir = app_layer_root()
local app_layer_hint = app_layer_root_dir .. "/target/release/app-layer-hint"

-- One file. Lines before the first [class] are the generic hotkeys.
-- A [class] section replaces the generic hotkeys for that window class.
local app_layer_file = app_layer_root_dir .. "/applications.conf"
-- Every letter, space, minus, and equal can be a layer key. The open config
-- decides which of them send a shortcut.
local app_layer_keys = {
  { bind = "space", name = "space" },
  { bind = "minus", name = "-" },
  { bind = "equal", name = "=" },
}
for code = string.byte("a"), string.byte("z") do
  local letter = string.char(code)
  table.insert(app_layer_keys, { bind = letter, name = letter })
end
local app_layer_map = {}
local app_layer_order = {}
local app_layer_warnings = {}
local app_layer_active = false
local app_layer_class = nil
local app_layer_target = nil
local app_layer_target_address = nil

local app_layer_parse = dofile(app_layer_root_dir .. "/parse.lua")
local app_layer_action_id = app_layer_parse.action_id

local function read_app_layer_file(class)
  local file = io.open(app_layer_file, "r")
  if not file then
    return app_layer_parse.empty(), {}
  end
  local body = file:read("*a")
  file:close()
  return app_layer_parse.parse(body)
end


local function app_layer_config_for(class)
  local generic, sections = read_app_layer_file(class)
  local map = {}
  local order = {}
  local warnings = {}
  local function add(text)
    table.insert(warnings, text)
  end
  local function put(key, command)
    if not map[key] then
      table.insert(order, key)
    end
    map[key] = { actions = command.actions, label = command.label }
  end
  for _, text in ipairs(generic.warnings) do
    add(text)
  end
  -- Only the focused window's section is applied, and its keys win.
  local chosen = class and sections[class]
  if chosen then
    for _, text in ipairs(chosen.warnings) do
      add(text)
    end
    for _, key in ipairs(chosen.order) do
      put(key, chosen.map[key])
    end
  end
  for _, key in ipairs(generic.order) do
    if not map[key] and not (chosen and chosen.disabled[key] and not chosen.map[key]) then
      put(key, generic.map[key])
    end
  end
  local seen = {}
  for _, key in ipairs(order) do
    local command = map[key]
    if command.label == "DISABLED" then
      -- A disabled key is not a conflict with anything else.
    else
      local sent = app_layer_action_id(command.actions)
      local other = seen[sent]
      if other and map[other].label ~= "DISABLED" then
        add(key .. " and " .. other .. " both send " .. sent)
      elseif not other then
        seen[sent] = key
      end
    end
  end
  return map, order, warnings
end

local function show_app_layer_hint()
  local spec = io.open(app_layer_spec, "w")
  if spec then
    spec:write("# ", app_layer_class or "generic", "\n")
    for _, warning in ipairs(app_layer_warnings) do
      spec:write("! ", warning, "\n")
    end
    for _, key in ipairs(app_layer_order) do
      local command = app_layer_map[key]
      if command then
        spec:write(key, "\t", command.label, "\n")
      end
    end
    spec:write("`\tedit\n")
    spec:close()
  end
  hl.exec_cmd(string.format("setsid %s %s %s", app_layer_hint, app_layer_pid, app_layer_spec))
end

local function hide_app_layer_hint()
  hl.exec_cmd(string.format(
    "if [[ -f %s ]]; then kill \"$(cat %s)\" 2>/dev/null; rm -f %s; fi",
    app_layer_pid,
    app_layer_pid,
    app_layer_pid
  ))
end

local function window_address(window)
  if not window or not window.address then
    return nil
  end
  return tostring(window.address):gsub("^address:", "")
end

local function app_layer_leave()
  if not app_layer_active then
    return
  end
  app_layer_active = false
  hide_app_layer_hint()
  hl.dispatch(hl.dsp.submap("reset"))
end

if app_layer_focus_sub then
  app_layer_focus_sub:remove()
end
app_layer_focus_sub = hl.on("window.active", function()
  if not app_layer_active then
    return
  end
  -- The event also fires for the window under the pointer. Only the
  -- keyboard-focused window should close the layer.
  local focused = hl.get_active_window()
  if not focused or focused.title == "Application layer" then
    return
  end
  if window_address(focused) == app_layer_target_address then
    return
  end
  app_layer_leave()
end)

local function app_layer_edit()
  app_layer_active = false
  hide_app_layer_hint()
  hl.exec_cmd("omarchy-launch-editor " .. string.format("%q", app_layer_file))
  hl.dispatch(hl.dsp.submap("reset"))
end

local function app_layer_send(layer_key)
  app_layer_active = false
  hide_app_layer_hint()
  local command = app_layer_map[layer_key]
  if command and app_layer_target then
    local path = (os.getenv("XDG_RUNTIME_DIR") or "/tmp") .. "/omarchy-app-layer-sequence"
    local file = io.open(path, "wb")
    if file then
      for _, action in ipairs(command.actions) do
        if action.text then
          file:write("T ", #action.text, "\n", action.text)
        else
          file:write("C ", action.shortcut, "\n")
        end
      end
      file:close()
      hl.dispatch(hl.dsp.submap("reset"))
      hl.exec_cmd(string.format(
        "%s send %s %s",
        string.format("%q", app_layer_hint),
        string.format("%q", app_layer_target),
        string.format("%q", path)
      ))
      return
    end
  end
  hl.dispatch(hl.dsp.submap("reset"))
end

hl.bind("ALT + SPACE", function()
  local window = hl.get_active_window()
  if window and window.title ~= "Application layer" and window.address then
    app_layer_target = "address:" .. window.address
    app_layer_target_address = window_address(window)
    app_layer_class = window.class
    app_layer_active = true
    app_layer_map, app_layer_order, app_layer_warnings = app_layer_config_for(window.class)
    show_app_layer_hint()
  else
    app_layer_target = nil
    app_layer_class = nil
    app_layer_map = {}
    app_layer_order = {}
    app_layer_warnings = {}
    app_layer_active = false
  end
  hl.dispatch(hl.dsp.submap("app-commands"))
end, { description = "Application layer" })

hl.define_submap("app-commands", "reset", function()
  hl.bind("ALT + SPACE", function()
    app_layer_active = false
    hide_app_layer_hint()
    hl.dispatch(hl.dsp.submap("reset"))
  end, { description = "Leave application layer" })
  hl.bind("escape", function()
    app_layer_active = false
    hide_app_layer_hint()
    hl.dispatch(hl.dsp.submap("reset"))
  end, { description = "Leave application layer" })

  local function bind_command(bind, name)
    hl.bind(bind, function()
      app_layer_send(name)
    end, { description = "Application layer: " .. name })
  end

  for _, key in ipairs(app_layer_keys) do
    bind_command(key.bind, key.name)
  end

  hl.bind("grave", app_layer_edit, { description = "Application layer: edit config" })

  hl.bind("catchall", function()
    app_layer_active = false
    hide_app_layer_hint()
    hl.dispatch(hl.dsp.submap("reset"))
  end)
end)

