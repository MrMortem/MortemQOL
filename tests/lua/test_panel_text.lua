-- Usage: luajit tests/lua/test_panel_text.lua
MQOL = {}
MQOL.format = dofile("src/format.lua")
local panel_text = dofile("src/panel_text.lua")

local function eq(a, b) assert(a == b, tostring(a) .. " ~= " .. tostring(b)) end

local function result(extra)
  local best = { hand = "Pair", expected = 64, floor = 64, ceiling = 64, blind_pct = 21.3,
    money_floor = 0, money_expected = 0, money_ceiling = 0 }
  local r = { best = best, gain = 0, gain_money = 0, order_exact = true, approximate = {}, blocked = false, blind_warning = false }
  for k, v in pairs(extra or {}) do
    if best[k] ~= nil then best[k] = v else r[k] = v end
  end
  return r
end

local function state(extra, selection, status)
  return { status = status or "ready", result = result(extra), selection = selection }
end

local tests = {}

function tests.score_mode_has_no_money_line()
  local t = panel_text.build(state({ money_expected = 3 }), "score", 0)
  eq(t.money, ""); eq(t.warning, "")
  eq(t.hand, "Pair"); eq(t.score, "64"); eq(t.percent, "21% of blind"); eq(t.range, ""); eq(t.status, "")
end

function tests.score_mode_range()
  eq(panel_text.build(state({ floor = 16, expected = 80, ceiling = 336 }), "score", 0).range, "Range 16 - 336")
end

function tests.score_mode_selection()
  eq(panel_text.build(state({}, { expected = 16, money_expected = 3 }), "score", 0).selection, "Selected: 16")
end

function tests.money_mode_shows_money_first()
  eq(panel_text.build(state({ money_floor = 3, money_expected = 3, money_ceiling = 3 }), "money", 0).money, "+$3")
  local t = panel_text.build(state({ money_floor = 0, money_expected = 4.5, money_ceiling = 20 }), "money", 0)
  eq(t.money, "+$4.50 ($0 to +$20)")
  eq(t.hand, "Pair"); eq(t.score, "64")
end

function tests.money_mode_selection_shows_both()
  eq(panel_text.build(state({}, { expected = 64, money_expected = 3 }), "money", 0).selection, "Selected: +$3, 64")
end

function tests.money_gain_preferred()
  eq(panel_text.build(state({ gain_money = 2, gain = 128 }), "money", 0).gain, "Reorder jokers: +$2")
  eq(panel_text.build(state({ gain_money = 0, gain = 128 }), "money", 0).gain, "Reorder jokers: +128")
end

function tests.score_mode_ignores_money_gain()
  eq(panel_text.build(state({ gain_money = 2, gain = 128 }), "score", 0).gain, "Reorder jokers: +128")
  eq(panel_text.build(state({ gain_money = 2, gain = 0 }), "score", 0).gain, "")
end

function tests.best_found_label_kept()
  eq(panel_text.build(state({ gain = 5, order_exact = false }), "score", 0).gain, "Reorder jokers (best found): +5")
end

function tests.blocked_label()
  eq(panel_text.build(state({ blocked = true }), "money", 0).hand, "Blocked: scores 0")
end

function tests.blind_warning_text()
  eq(panel_text.build(state({ blind_warning = true }), "money", 0).warning, "Won't beat the blind")
end

function tests.approximate_and_mismatch_shown_together()
  eq(panel_text.build(state({ approximate = { "lobby" } }), "score", 2).status, "2 score mismatches logged | Approximate (hover)")
  eq(panel_text.build(state({}), "score", 1).status, "1 score mismatch logged")
end

function tests.calculating_status()
  eq(panel_text.build(state({}, nil, "calculating"), "score", 0).status, "Calculating...")
end

function tests.naneinf_gain_is_safe()
  eq(panel_text.build(state({ gain = "naneinf" }), "score", 0).gain, "Reorder jokers: +naneinf")
  eq(panel_text.build(state({ gain_money = "naneinf" }), "money", 0).gain, "Reorder jokers: +naneinf")
end

function tests.unavailable_status()
  local t = panel_text.build(state({}, { expected = 5, money_expected = 1 }, "unavailable"), "money", 0)
  eq(t.status, "Engine unavailable"); eq(t.hand, ""); eq(t.money, ""); eq(t.selection, ""); eq(t.gain, "")
end

function tests.no_play_message()
  local t = panel_text.build({ status = "ready", result = { best = nil, approximate = {} } }, "score", 0)
  eq(t.hand, "No scoring play")
end

local failed, names = 0, {}
for name in pairs(tests) do names[#names + 1] = name end
table.sort(names)
for _, name in ipairs(names) do
  local ok, err = pcall(tests[name])
  if not ok then failed = failed + 1; print("FAIL " .. name .. ": " .. tostring(err)) end
end
assert(failed == 0, failed .. " failed")
print("test_panel_text: ok (" .. #names .. ")")
