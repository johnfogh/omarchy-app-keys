local parse = dofile("parse.lua")

local failed = 0
local function check(name, ok)
  if ok then
    io.write("ok  ", name, "\n")
  else
    failed = failed + 1
    io.write("FAIL ", name, "\n")
  end
end

local generic, sections = parse.parse([[
f, ctrl+f, find
n, ctrl+n, new

[foot]
w, super+w, "done", quit
u, "/usage", "\r", usage
s, ctrl+s,
x, "a, b", note

[brave-browser]
n, ctrl+t, "\x1b", new tab
]])

local find = generic.map.f
check("generic chord keeps its label", find and find.label == "find" and find.actions[1].shortcut == "ctrl+f")

local quit = sections.foot.map.w
check("last field is the label", quit and quit.label == "quit")
check("chord then string are both sent",
  quit and quit.actions[1].shortcut == "super+w" and quit.actions[2].text == "done")

local usage = sections.foot.map.u
check("several strings stay in order",
  usage and usage.actions[1].text == "/usage" and usage.actions[2].text == "\r" and usage.label == "usage")

check("blank label disables the key", sections.foot.disabled.s and not sections.foot.map.s)

local disabled = parse.parse("f, not-a-chord, DISABLED\n., ctrl+f, DISABLED\n")
check("DISABLED hides the key without a warning", disabled.disabled.f and not disabled.map.f and #disabled.warnings == 0)

local note = sections.foot.map.x
check("comma inside quotes is part of the string", note and note.actions[1].text == "a, b" and note.label == "note")

local tab = sections["brave-browser"].map.n
check("escape in a quoted string is decoded",
  tab and tab.actions[1].shortcut == "ctrl+t" and tab.actions[2].text == "\27" and tab.label == "new tab")

check("generic keys remain for other applications", generic.map.n.label == "new")

local _, all_sections = parse.parse([[
f, ctrl+f, find

[foot]
w, super+w, quit

[brave-browser]
n, ctrl+t, new tab
]])
check("every application section is parsed", all_sections.foot.map.w.label == "quit" and all_sections["brave-browser"].map.n.label == "new tab")

check("shift+f canonical name", parse.key_name("SHIFT+F") == "shift+f")
check("ctrl+alt+f1 canonical order", parse.key_name("alt+ctrl+f1") == "ctrl+alt+f1")
check("control alias", parse.key_name("control+a") == "ctrl+a")
check("minus alias", parse.key_name("shift+minus") == "shift+-")
check("equal alias", parse.key_name("ctrl+equal") == "ctrl+=")
check("alt+space reserved", parse.key_name("alt+space") == nil)
check("f1 reserved for configure", parse.key_name("f1") == nil)
check("shift+f1 still allowed", parse.key_name("shift+f1") == "shift+f1")
check("super rejected on layer key", parse.key_name("super+w") == nil)
check("bind string for ctrl+f", parse.bind_string("ctrl+f") == "CTRL + f")
check("bind string for f12", parse.bind_string("f12") == "F12")
check("bind string for shift+-", parse.bind_string("shift+-") == "SHIFT + minus")

local modded = parse.parse([[
shift+f, ctrl+shift+f, find selection
f2, "help", help
ctrl+f1, ctrl+shift+p, palette
]])
check("modified layer key is stored", modded.map["shift+f"] and modded.map["shift+f"].label == "find selection")
check("function layer key is stored", modded.map.f2 and modded.map.f2.actions[1].text == "help")
check("modified function key is stored", modded.map["ctrl+f1"] and modded.map["ctrl+f1"].label == "palette")

local keys = parse.layer_keys()
local names = {}
for _, key in ipairs(keys) do
  names[key.name] = key.bind
end
check("layer_keys includes letters", names.f == "f")
check("layer_keys omits bare f1", names.f1 == nil)
check("layer_keys includes other function keys", names.f2 == "F2")
check("layer_keys includes modifiers", names["ctrl+f"] == "CTRL + f" and names["shift+f2"] == "SHIFT + F2")
check("layer_keys omits alt+space", names["alt+space"] == nil)

if failed > 0 then
  os.exit(1)
end
