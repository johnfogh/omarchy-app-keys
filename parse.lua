-- Parser for applications.conf. The last comma-separated field is the label.
-- Every field between the layer key and the label is sent, in order.

local M = {}

function M.key_name(key)
  key = key:match("^%s*(.-)%s*$"):lower()
  if key == "minus" then
    return "-"
  end
  if key == "equal" then
    return "="
  end
  return key
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
  if not (key:match("^[%a]+$") or key == "-" or key == "=") then
    warn(section, "ignored " .. key .. ", unsupported layer key")
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
      table.insert(fields, { quoted = false, value = text:sub(start, i - 1):match("^%s*(.-)%s*$") })
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
    local row = line:match("^%s*(.-)%s*$")
    local header = row:match("^%[([^%]]+)%]$")
    if header then
      header = header:match("^%s*(.-)%s*$")
      current = M.empty()
      sections[header] = current
    elseif row ~= "" and not row:match("^#") then
      local fields = M.fields(row)
      if #fields >= 2 then
        local key = M.key_name(fields[1].value)
        local label = fields[#fields].value:match("^%s*(.-)%s*$")
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
  return generic, sections
end

return M
