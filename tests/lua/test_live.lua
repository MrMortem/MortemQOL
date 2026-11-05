-- Usage: luajit tests/lua/test_live.lua
package.path = "./?.lua;./tests/lua/?.lua;" .. package.path
local fake = require("fake_game")

local function setup(overrides)
  G = fake.new()
  MQOL = { warnings = {} }
  MQOL.settings = dofile("src/settings.lua")
  MQOL.settings._save = function() end
  MQOL.settings.init({})
  MQOL.json = dofile("src/json.lua")
  MQOL.snapshot = dofile("src/snapshot.lua")
  local calls = { starts = 0, scores = 0, polls = {} }
  MQOL.bridge = {
    available = function() return true end,
    search_start = function() calls.starts = calls.starts + 1; return calls.starts end,
    search_poll = function(id) calls.polls[#calls.polls + 1] = id; return 0, nil end,
    score = function() calls.scores = calls.scores + 1; return 1, '{"expected":5,"legal":true}' end,
  }
  for k, v in pairs(overrides or {}) do MQOL.bridge[k] = v end
  MQOL.live = dofile("src/live.lua")
  MQOL.live._reset()
  return calls
end

local RESULT = '{"done":true,"best":{"play":[1,2],"expected":64},"joker_order":[]}'

local tests = {}

function tests.search_starts_during_hand_selection()
  local calls = setup()
  MQOL.live.update(1)
  assert(calls.starts == 1)
  assert(MQOL.live.state().status == "calculating")
end

function tests.no_search_outside_hand_selection()
  local calls = setup()
  G.STATE = G.STATES.HAND_PLAYED
  MQOL.live.update(1)
  assert(calls.starts == 0)
  assert(MQOL.live.state().status == "idle")
end

function tests.no_new_search_when_json_unchanged()
  local calls = setup()
  MQOL.live.update(1)
  MQOL.live.update(1)
  assert(calls.starts == 1)
end

function tests.state_change_starts_new_search_and_clears_result()
  local calls = setup({ search_poll = function(id)
    if id == 1 then return 1, RESULT end
    return 0, nil
  end })
  MQOL.live.update(1)
  assert(MQOL.live.state().result)
  table.remove(G.hand.cards, 3)
  MQOL.live.update(1)
  assert(calls.starts == 2)
  assert(MQOL.live.state().result == nil, "stale result shown")
end

function tests.rebuild_is_throttled()
  local calls = setup()
  MQOL.live.update(1)
  table.remove(G.hand.cards, 3)
  MQOL.live.update(0.01)
  assert(calls.starts == 1)
  MQOL.live.update(0.2)
  assert(calls.starts == 2)
end

function tests.provisional_result_is_shown_while_running()
  setup({ search_poll = function() return 0, '{"done":false,"best":{"play":[1],"expected":16}}' end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  local st = MQOL.live.state()
  assert(st.status == "calculating" and st.result.best.expected == 16)
end

function tests.finished_result_sets_ready_and_stops_polling()
  local polls = 0
  setup({ search_poll = function() polls = polls + 1; return 1, RESULT end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  MQOL.live.update(0.01)
  assert(MQOL.live.state().status == "ready")
  assert(polls == 1)
end

function tests.stale_poll_ignored()
  setup({ search_poll = function() return -3, nil end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  assert(MQOL.live.state().result == nil)
end

function tests.engine_error_shows_unavailable_then_retries_on_change()
  local calls = setup({ search_poll = function() return -1, "boom" end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  assert(MQOL.live.state().status == "unavailable")
  table.remove(G.hand.cards, 3)
  MQOL.live.update(1)
  assert(calls.starts == 2)
end

function tests.unavailable_bridge_never_builds()
  local calls = setup({ available = function() return false end })
  MQOL.snapshot.build = function() error("should not build") end
  MQOL.live.update(1)
  assert(MQOL.live.state().status == "unavailable")
  assert(calls.starts == 0)
end

function tests.error_in_build_sets_unavailable_not_raise()
  setup()
  MQOL.snapshot.build = function() error("broken") end
  MQOL.live.update(1)
  assert(MQOL.live.state().status == "unavailable")
end

function tests.selection_is_scored_when_it_changes()
  local calls = setup()
  G.hand.highlighted = { G.hand.cards[1] }
  MQOL.live.update(1)
  assert(calls.scores == 1)
  assert(MQOL.live.state().selection.expected == 5)
  MQOL.live.update(0.2)
  assert(calls.scores == 1, "unchanged selection should not rescore")
  G.hand.highlighted = {}
  MQOL.live.update(0.2)
  assert(MQOL.live.state().selection == nil)
end

function tests.select_best_highlights_recommended_cards()
  setup({ search_poll = function() return 1, RESULT end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  assert(MQOL.live.select_best() == true)
  assert(#G.hand.highlighted == 2 and G.hand.highlighted[1].sort_id == 1)
end

function tests.select_best_noop_when_card_gone()
  setup({ search_poll = function() return 1, RESULT end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  G.hand.highlighted = { G.hand.cards[3] }
  table.remove(G.hand.cards, 1)
  assert(MQOL.live.select_best() == false)
  assert(#G.hand.highlighted == 1, "selection should be untouched")
end

function tests.select_best_noop_outside_hand_selection()
  setup({ search_poll = function() return 1, RESULT end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  G.STATE = G.STATES.HAND_PLAYED
  assert(MQOL.live.select_best() == false)
end

function tests.select_best_noop_without_result()
  setup()
  assert(MQOL.live.select_best() == false)
end

function tests.select_best_skips_already_highlighted_forced_card()
  setup({ search_poll = function() return 1, RESULT end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  G.hand.cards[1].ability.forced_selection = true
  function G.hand:unhighlight_all() self.highlighted = { G.hand.cards[1] } end
  assert(MQOL.live.select_best() == true)
  assert(#G.hand.highlighted == 2)
end

function tests.leaving_hand_selection_clears_result()
  setup({ search_poll = function() return 1, RESULT end })
  MQOL.live.update(1)
  MQOL.live.update(0.01)
  G.STATE = G.STATES.HAND_PLAYED
  MQOL.live.update(1)
  assert(MQOL.live.state().result == nil)
end

function tests.assist_off_clears_and_stops()
  local calls = setup({ search_poll = function() return 1, RESULT end })
  MQOL.live.update(1)
  assert(MQOL.live.state().result)
  G.hand.highlighted = { G.hand.cards[1] }
  MQOL.settings.set_assist(false)
  local starts, scores = calls.starts, calls.scores
  MQOL.live.update(1)
  MQOL.live.update(1)
  local st = MQOL.live.state()
  assert(st.status == "idle" and st.result == nil and st.selection == nil)
  assert(calls.starts == starts and calls.scores == scores)
end

function tests.assist_off_does_not_build()
  setup()
  MQOL.settings.set_assist(false)
  MQOL.snapshot.build = function() error("should not build") end
  MQOL.live.update(1)
  assert(MQOL.live.state().status == "idle")
end

function tests.assist_on_restarts_search()
  local calls = setup()
  MQOL.settings.set_assist(false)
  MQOL.live.update(1)
  MQOL.settings.set_assist(true)
  MQOL.live.update(0.01)
  assert(calls.starts == 1 and MQOL.live.state().status == "calculating")
end

function tests.mode_change_starts_new_search_and_clears_result()
  local polled = {}
  local calls = setup({ search_poll = function(id)
    polled[#polled + 1] = id
    if id == 1 then return 1, RESULT end
    return 0, nil
  end })
  MQOL.live.update(1)
  assert(MQOL.live.state().result)
  MQOL.settings.set_mode("money")
  MQOL.live.update(0.01)
  assert(calls.starts == 2, "mode change should search at once")
  assert(MQOL.live.state().result == nil, "old mode's result shown")
  assert(polled[#polled] == 2)
end

function tests.search_json_contains_mode()
  local sent
  setup({ search_start = function(json) sent = json; return 1 end })
  MQOL.settings.set_mode("money")
  MQOL.live.update(1)
  assert(sent:find('"objective":"money"', 1, true))
end

local failed = 0
local names = {}
for name in pairs(tests) do names[#names + 1] = name end
table.sort(names)
for _, name in ipairs(names) do
  local ok, err = pcall(tests[name])
  if not ok then failed = failed + 1; print("FAIL " .. name .. ": " .. tostring(err)) end
end
assert(failed == 0, failed .. " failed")
print("test_live: ok (" .. #names .. ")")
