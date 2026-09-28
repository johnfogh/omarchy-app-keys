-- Parser for applications.conf. The last comma-separated field is the label.
-- Every field between the layer key and the label is sent, in order.
-- Layer keys may be a letter, space, -, =, f1–f12, optionally with ctrl/alt/shift.

local M = {}

local LAYER_MODS = { ctrl = true, alt = true, shift = true }
local MOD_ORDER = { "ctrl", "alt", "shift" }

local function trim(text)
  return text:match("^%s*(.-)%s*$")
end

local function is_function_key(base)
  local n = base:match("^f(%d+)$")
  if not n then
    return false
  end
  n = tonumber(n)
  return n >= 1 and n <= 12
end

local function is_base_key(base)
  if base == "-" or base == "=" or base == "space" then
    return true
  end
  if base:match("^%l$") then
    return true
  end
  return is_function_key(base)
end

--- Canonical layer-key name: mods in ctrl, alt, shift order, then the base.
--- Accepts minus/equal/control aliases. Rejects super and unknown bases.
function M.key_name(key)
  key = trim(key):lower()
  if key == "" then
    return nil, "empty layer key"
  end

  local parts = {}
  for part in key:gmatch("[^%+]+") do
    table.insert(parts, trim(part))
  end
  if #parts == 0 then
    return nil, "empty layer key"
  end

  local mods = {}
  local seen = {}
  local base = nil
  for index, part in ipairs(parts) do
    if part == "control" then
      part = "ctrl"
    elseif part == "minus" then
      part = "-"
    elseif part == "equal" then
      part = "="
    end
    if index < #parts or LAYER_MODS[part] then
      if part == "super" or part == "super_l" then
        return nil, "super is not allowed on a layer key"
      end
      if not LAYER_MODS[part] then
        return nil, "unsupported layer modifier " .. part
      end
      if seen[part] then
        return nil, "duplicate modifier " .. part
      end
      seen[part] = true
      table.insert(mods, part)
    else
      base = part
    end
  end

  if not base then
    return nil, "layer key needs a base key"
  end
  if not is_base_key(base) then
    return nil, "unsupported layer key " .. base
  end

  table.sort(mods, function(a, b)
    local ia, ib = 0, 0
    for i, name in ipairs(MOD_ORDER) do
      if name == a then
        ia = i
      end
      if name == b then
        ib = i
      end
    end
    return ia < ib
  end)

  -- Alt+Space toggles the layer; keep it reserved.
  if base == "space" and #mods == 1 and mods[1] == "alt" then
    return nil, "alt+space is reserved"
  end

  if #mods == 0 then
    return base
  end
  return table.concat(mods, "+") .. "+" .. base
end

--- Hyprland bind string for a canonical layer key (e.g. "ctrl+f" → "CTRL + f").
function M.bind_string(canonical)
  local parts = {}
  for part in canonical:gmatch("[^%+]+") do
    table.insert(parts, part)
  end
  local base = parts[#parts]
  local tokens = {}
  for i = 1, #parts - 1 do
    table.insert(tokens, parts[i]:upper())
  end
  if base == "-" then
    table.insert(tokens, "minus")
  elseif base == "=" then
    table.insert(tokens, "equal")
  elseif is_function_key(base) then
    table.insert(tokens, base:upper())
  elseif base == "space" then
    table.insert(tokens, "space")
  else
    table.insert(tokens, base)
  end
  return table.concat(tokens, " + ")
end

function M.action_id(actions)
  local parts = {}
  for _, action in ipairs(actions) do
    table.insert(parts, action.text and ("text:" .. action.text) or action.shortcut)
  end
  return table.concat(parts, " | ")
end

local function warn(section, text)
  table.insert(section.warnings, text)
end

local function drop(section, key)
  if not section.map[key] then
    return
  end
  section.map[key] = nil
  for index = #section.order, 1, -1 do
    if section.order[index] == key then
      table.remove(section.order, index)
    end
  end
end

function M.apply(section, key, actions, label)
  if label == "DISABLED" then
    section.disabled[key] = true
    drop(section, key)
    return
  end
  local invalid = nil
  for _, action in ipairs(actions) do
    if not action.text and not action.shortcut:match("^[%w+=%-]+$") then
      invalid = action.shortcut
    end
  end
  local command = { actions = actions, label = label }
  if label == "" then
    section.disabled[key] = true
    drop(section, key)
  elseif #actions == 0 or invalid then
    warn(section, key .. " has an unusable keystroke" .. (invalid and (" " .. invalid) or ""))
    section.disabled[key] = true
    drop(section, key)
  elseif section.map[key] then
    local existing = section.map[key]
    if M.action_id(existing.actions) ~= M.action_id(actions) or existing.label ~= label then
      warn(section, key .. " conflicts: " .. existing.label .. " and " .. label)
    end
  else
    section.disabled[key] = nil
    section.map[key] = command
    table.insert(section.order, key)
  end
end

function M.fields(text)
  local fields = {}
  local i = 1
  local n = #text
  local function skip_ws()
    while i <= n and text:sub(i, i):match("%s") do
      i = i + 1
    end
  end
  local function unescape(body, quote)
    local out = {}
    local j = 1
    while j <= #body do
      local ch = body:sub(j, j)
      if ch == "\\" and quote == '"' then
        local nxt = body:sub(j + 1, j + 1)
        if nxt == "n" then
          table.insert(out, "\n")
          j = j + 2
        elseif nxt == "t" then
          table.insert(out, "\t")
          j = j + 2
        elseif nxt == "r" then
          table.insert(out, "\r")
          j = j + 2
        elseif nxt == "\\" or nxt == '"' then
          table.insert(out, nxt)
          j = j + 2
        elseif nxt == "x" and body:sub(j + 2, j + 3):match("^%x%x$") then
          table.insert(out, string.char(tonumber(body:sub(j + 2, j + 3), 16)))
          j = j + 4
        else
          table.insert(out, nxt)
          j = j + 2
        end
      else
        table.insert(out, ch)
        j = j + 1
      end
    end
    return table.concat(out)
  end
  while i <= n do
    skip_ws()
    if i > n then
      break
    end
    local c = text:sub(i, i)
    if c == '"' or c == "'" then
      local quote = c
      local start = i + 1
      i = i + 1
      while i <= n do
        if text:sub(i, i) == "\\" and quote == '"' then
          i = i + 2
        elseif text:sub(i, i) == quote then
          break
        else
          i = i + 1
        end
      end
      local raw = text:sub(start, i - 1)
      i = i + 1
      table.insert(fields, { quoted = true, value = unescape(raw, quote) })
      skip_ws()
      if text:sub(i, i) == "," then
        i = i + 1
      end
    else
      local start = i
      while i <= n and text:sub(i, i) ~= "," do
        i = i + 1
      end
      table.insert(fields, { quoted = false, value = trim(text:sub(start, i - 1)) })
      if text:sub(i, i) == "," then
        i = i + 1
      end
    end
  end
  return fields
end

function M.empty()
  return { map = {}, order = {}, warnings = {}, disabled = {} }
end

function M.parse(text)
  local generic = M.empty()
  local sections = {}
  local current = generic
  for line in (text .. "\n"):gmatch("(.-)\n") do
    local row = trim(line)
    local header = row:match("^%[([^%]]+)%]$")
    if header then
      header = trim(header)
      current = M.empty()
      sections[header] = current
    elseif row ~= "" and not row:match("^#") then
      local fields = M.fields(row)
      if #fields >= 2 then
        local label = trim(fields[#fields].value)
        local key, err = M.key_name(fields[1].value)
        if not key then
          -- DISABLED is a silent off switch even for unusable layer keys.
          if label ~= "DISABLED" then
            warn(current, "ignored " .. fields[1].value .. ", " .. (err or "unsupported layer key"))
          end
        else
          local actions = {}
          for index = 2, #fields - 1 do
            local sent = fields[index]
            if sent.quoted then
              table.insert(actions, { text = sent.value })
            else
              table.insert(actions, { shortcut = sent.value:lower() })
            end
          end
          M.apply(current, key, actions, label)
        end
      end
    end
  end
  return generic, sections
end

--- Every layer key the submap should bind: bases × optional ctrl/alt/shift.
function M.layer_keys()
  local bases = { "space", "-", "=" }
  for code = string.byte("a"), string.byte("z") do
    table.insert(bases, string.char(code))
  end
  for n = 1, 12 do
    table.insert(bases, "f" .. n)
  end

  local mod_sets = {
    {},
    { "shift" },
    { "ctrl" },
    { "alt" },
    { "ctrl", "shift" },
    { "alt", "shift" },
    { "ctrl", "alt" },
    { "ctrl", "alt", "shift" },
  }

  local keys = {}
  for _, mods in ipairs(mod_sets) do
    for _, base in ipairs(bases) do
      local canonical
      if #mods == 0 then
        canonical = base
      else
        canonical = table.concat(mods, "+") .. "+" .. base
      end
      if not (base == "space" and #mods == 1 and mods[1] == "alt") then
        table.insert(keys, {
          name = canonical,
          bind = M.bind_string(canonical),
        })
      end
    end
  end
  return keys
end

return M
