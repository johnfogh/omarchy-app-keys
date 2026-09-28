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

if failed > 0 then
  os.exit(1)
end
