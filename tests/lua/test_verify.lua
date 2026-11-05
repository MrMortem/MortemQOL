-- Usage: luajit tests/lua/test_verify.lua
local fake = dofile("tests/lua/fake_game.lua")

local function setup(score_json, status)
  G = fake.new()
  MQOL = {}
  MQOL.settings = dofile("src/settings.lua")
  MQOL.json = dofile("src/json.lua")
  MQOL.snapshot = dofile("src/snapshot.lua")
  MQOL.bridge = {
    available = function() return true end,
    score = function() return status or 1, score_json end,
  }
  MQOL.verify = dofile("src/verify.lua")
  local written = {}
  MQOL.verify._write = function(file, text) written[#written + 1] = { file = file, text = text } end
  return written
end

local function pred(extra)
  local p = { floor = 64, expected = 64, ceiling = 64, legal = true, has_chance = false, approximate = {} }
  for k, v in pairs(extra or {}) do p[k] = v end
  return p
end

local tests = {}

local function money_pred(extra)
  local p = pred({ money_floor = 3, money_expected = 3, money_ceiling = 3 })
  for k, v in pairs(extra or {}) do p[k] = v end
  return p
end

local MONEY_JSON = '{"floor":64,"expected":64,"ceiling":64,"legal":true,"has_chance":false,"approximate":[],"money_floor":3,"money_expected":3,"money_ceiling":3}'

local function play_and_score(json)
  local written = setup(json or MONEY_JSON)
  G.GAME.dollars = 10
  G.hand.highlighted = { G.hand.cards[1] }
  MQOL.verify.before_play()
  G.STATE = G.STATES.HAND_PLAYED
  MQOL.verify.after_score(64)
  return written
end

function tests.money_exact_match()
  setup()
  assert(MQOL.verify.compare_money(money_pred(), 3) == "match")
  assert(MQOL.verify.compare_money(money_pred(), 3 + 1e-12) == "match")
end

function tests.money_exact_mismatch()
  setup()
  assert(MQOL.verify.compare_money(money_pred(), 4) == "mismatch")
end

function tests.money_range_match()
  setup()
  local p = money_pred({ money_floor = 0, money_expected = 1.3, money_ceiling = 20 })
  assert(MQOL.verify.compare_money(p, 0) == "match")
  assert(MQOL.verify.compare_money(p, 20) == "match")
end

function tests.money_outside_range_mismatches()
  setup()
  local p = money_pred({ money_floor = 0, money_expected = 1.3, money_ceiling = 20 })
  assert(MQOL.verify.compare_money(p, 21) == "mismatch")
  assert(MQOL.verify.compare_money(p, -1) == "mismatch")
end

function tests.money_approximate_never_mismatches()
  setup()
  assert(MQOL.verify.compare_money(money_pred({ approximate = { "lobby" } }), 99) == "approximate")
end

function tests.tick_waits_for_hand_played_to_end()
  play_and_score()
  local before = MQOL.verify.counts().match
  G.GAME.dollars = 13
  MQOL.verify.tick()
  assert(MQOL.verify.counts().match == before, "compared too early")
  G.STATE = G.STATES.DRAW_TO_HAND
  MQOL.verify.tick()
  assert(MQOL.verify.counts().match == before + 1)
  MQOL.verify.tick()
  assert(MQOL.verify.counts().match == before + 1, "compared twice")
end

function tests.money_mismatch_writes_line_with_actual_money()
  local written = play_and_score()
  G.GAME.dollars = 99
  G.STATE = G.STATES.NEW_ROUND
  MQOL.verify.tick()
  assert(#written == 1 and written[1].file == "verify.jsonl")
  local line = MQOL.json.decode(written[1].text)
  assert(line.actual_money == 89 and line.prediction.money_expected == 3 and line.actual == nil)
  assert(MQOL.verify.counts().mismatch == 1)
end

function tests.new_play_discards_uncompared_money()
  play_and_score()
  local before = MQOL.verify.counts().match
  MQOL.verify.before_play()
  G.STATE = G.STATES.DRAW_TO_HAND
  G.GAME.dollars = 500
  MQOL.verify.tick()
  assert(MQOL.verify.counts().mismatch == 0, "compared a stale balance")
  assert(MQOL.verify.counts().match == before)
end

function tests.quit_mid_hand_discards_money_comparison()
  local written = play_and_score()
  -- Game:delete_run sets the state to -1, drops unpaid money, and a new game starts.
  G.STATE = -1
  G.GAME = { dollars = 4 }
  MQOL.verify.tick()
  G.STATE = G.STATES.DRAW_TO_HAND
  MQOL.verify.tick()
  assert(MQOL.verify.counts().mismatch == 0, "false mismatch after quitting")
  assert(#written == 0)
end

function tests.leaving_the_run_discards_money_comparison()
  play_and_score()
  G.STAGE = G.STAGES.MAIN_MENU
  G.STATE = G.STATES.DRAW_TO_HAND
  G.GAME = { dollars = 4 }
  MQOL.verify.tick()
  assert(MQOL.verify.counts().mismatch == 0)
end

function tests.round_end_state_still_compares()
  play_and_score()
  G.GAME.dollars = 13
  G.STATE = G.STATES.NEW_ROUND
  MQOL.verify.tick()
  assert(MQOL.verify.counts().match == 2)
end

function tests.assist_off_records_nothing()
  setup(MONEY_JSON)
  MQOL.settings.init({ assist = false })
  G.hand.highlighted = { G.hand.cards[1] }
  MQOL.verify.before_play()
  MQOL.verify.after_score(1)
  assert(MQOL.verify.counts().mismatch == 0 and MQOL.verify.counts().match == 0)
end

function tests.tick_finishes_a_play_when_assist_goes_off()
  play_and_score()
  local before = MQOL.verify.counts().match
  MQOL.settings.init({ assist = false })
  G.GAME.dollars = 13
  G.STATE = G.STATES.DRAW_TO_HAND
  MQOL.verify.tick()
  assert(MQOL.verify.counts().match == before + 1)
end

function tests.tick_error_does_not_raise()
  play_and_score()
  G.GAME = nil
  G.STATE = G.STATES.SELECTING_HAND
  MQOL.verify.tick()
end

function tests.exact_match_within_1e_9()
  setup()
  assert(MQOL.verify.compare(pred(), 64) == "match")
  assert(MQOL.verify.compare(pred(), 64 * (1 + 1e-10)) == "match")
end

function tests.exact_mismatch_outside_1e_9()
  setup()
  assert(MQOL.verify.compare(pred(), 64.01) == "mismatch")
  assert(MQOL.verify.compare(pred(), 0) == "mismatch")
end

function tests.chance_inside_range_matches()
  setup()
  local p = pred({ floor = 16, expected = 80, ceiling = 336, has_chance = true })
  assert(MQOL.verify.compare(p, 16) == "match")
  assert(MQOL.verify.compare(p, 336) == "match")
  assert(MQOL.verify.compare(p, 100) == "match")
end

function tests.chance_outside_range_mismatches()
  setup()
  local p = pred({ floor = 16, expected = 80, ceiling = 336, has_chance = true })
  assert(MQOL.verify.compare(p, 15) == "mismatch")
  assert(MQOL.verify.compare(p, 337) == "mismatch")
end

function tests.approximate_never_counts_as_mismatch()
  setup()
  assert(MQOL.verify.compare(pred({ approximate = { "lobby" } }), 1) == "approximate")
end

function tests.illegal_prediction_and_zero_actual_match()
  setup()
  local p = pred({ floor = 0, expected = 0, ceiling = 0, legal = false })
  assert(MQOL.verify.compare(p, 0) == "match")
  assert(MQOL.verify.compare(p, 5) == "mismatch")
end

function tests.naneinf_prediction_with_infinite_actual_matches()
  setup()
  local p = pred({ floor = "naneinf", expected = "naneinf", ceiling = "naneinf" })
  assert(MQOL.verify.compare(p, math.huge) == "match")
  assert(MQOL.verify.compare(p, 0 / 0) == "match")
  assert(MQOL.verify.compare(p, 100) == "mismatch")
end

function tests.finite_prediction_with_infinite_actual_mismatches()
  setup()
  assert(MQOL.verify.compare(pred(), math.huge) == "mismatch")
end

function tests.mismatch_writes_replayable_line()
  local written = setup('{"floor":64,"expected":64,"ceiling":64,"legal":true,"has_chance":false,"approximate":[]}')
  G.hand.highlighted = { G.hand.cards[1], G.hand.cards[2] }
  MQOL.verify.before_play()
  MQOL.verify.after_score(99)
  assert(#written == 1 and written[1].file == "verify.jsonl")
  local line = MQOL.json.decode(written[1].text)
  assert(line.actual == 99 and line.prediction.expected == 64)
  assert(line.play[1] == 1 and line.play[2] == 2)
  assert(line.snapshot.schema == 1)
  assert(written[1].text:sub(-1) == "\n")
  assert(MQOL.verify.counts().mismatch == 1)
end

function tests.match_writes_nothing_and_counts()
  local written = setup('{"floor":64,"expected":64,"ceiling":64,"legal":true,"has_chance":false,"approximate":[]}')
  G.hand.highlighted = { G.hand.cards[1] }
  MQOL.verify.before_play()
  MQOL.verify.after_score(64)
  assert(#written == 0)
  assert(MQOL.verify.counts().match == 1)
end

function tests.approximate_difference_logged_separately()
  local written = setup('{"floor":64,"expected":64,"ceiling":64,"legal":true,"has_chance":false,"approximate":["lobby"]}')
  G.hand.highlighted = { G.hand.cards[1] }
  MQOL.verify.before_play()
  MQOL.verify.after_score(70)
  assert(#written == 1 and written[1].file == "verify_approximate.jsonl")
  assert(MQOL.verify.counts().approximate == 1 and MQOL.verify.counts().mismatch == 0)
end

function tests.after_score_without_prediction_is_ignored()
  setup()
  MQOL.verify.after_score(5)
  assert(MQOL.verify.counts().match == 0 and MQOL.verify.counts().mismatch == 0)
end

function tests.prediction_is_used_only_once()
  setup('{"floor":64,"expected":64,"ceiling":64,"legal":true,"has_chance":false,"approximate":[]}')
  G.hand.highlighted = { G.hand.cards[1] }
  MQOL.verify.before_play()
  MQOL.verify.after_score(64)
  MQOL.verify.after_score(64)
  assert(MQOL.verify.counts().match == 1)
end

function tests.verify_error_does_not_raise()
  setup()
  MQOL.snapshot.build = function() error("broken") end
  MQOL.verify.before_play()
  MQOL.verify.after_score(5)
  assert(MQOL.verify.counts().mismatch == 0)
end

function tests.engine_error_skips_comparison()
  setup("boom", -1)
  G.hand.highlighted = { G.hand.cards[1] }
  MQOL.verify.before_play()
  MQOL.verify.after_score(5)
  assert(MQOL.verify.counts().mismatch == 0)
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
print("test_verify: ok (" .. #names .. ")")
