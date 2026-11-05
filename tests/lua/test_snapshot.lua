-- Usage: luajit tests/lua/test_snapshot.lua
package.path = "./?.lua;./tests/lua/?.lua;" .. package.path
local fake = require("fake_game")

MQOL = {}
MQOL.json = dofile("src/json.lua")
MQOL.snapshot = dofile("src/snapshot.lua")

local function deep_copy(v, seen)
  if type(v) ~= "table" then return v end
  seen = seen or {}
  if seen[v] then return seen[v] end
  local out = {}
  seen[v] = out
  for k, x in next, v do out[k] = deep_copy(x, seen) end
  return out
end

local function deep_equal(a, b, seen)
  if type(a) ~= type(b) then return false end
  if type(a) ~= "table" then return a == b end
  seen = seen or {}
  if seen[a] then return true end
  seen[a] = true
  for k, x in next, a do if not deep_equal(x, rawget(b, k), seen) then return false end end
  for k in next, b do if rawget(a, k) == nil then return false end end
  return true
end

local tests = {}

function tests.snapshot_never_reads_deck_order()
  G = fake.new()
  local s = MQOL.snapshot.build()
  assert(s.deck.remaining == 44)
  assert(s.deck.total == 52)
  assert(s.deck.starting == 52)
end

function tests.fields_map_from_game_state()
  G = fake.new()
  local s = MQOL.snapshot.build()
  assert(s.schema == 1)
  assert(s.hand[1].id == 1 and s.hand[1].rank == 14 and s.hand[1].suit == "Spades")
  assert(s.levels.Pair.played_round == 0 and s.levels.Pair.l_chips == 15)
  assert(s.blind.key == "bl_small" and s.blind.target == 300 and s.blind.disabled == false)
  assert(s.round.hands_left == 4 and s.round.dollars == 4 and s.round.tarot_used == 2)
  assert(s.round.idol.rank == 14 and s.round.ancient_suit == "Hearts")
  assert(s.round.most_played == "High Card" and s.round.round_hands == 0)
  assert(s.deck.back == "b_red" and s.lobby == false)
end

function tests.enhancement_edition_seal_and_perma()
  G = fake.new()
  local c = G.hand.cards[1]
  c.config.center = { key = "m_glass", set = "Enhanced" }
  c.ability.set = "Enhanced"
  c.edition = { key = "e_holo" }
  c.seal = "Red"
  c.ability.perma_bonus = 5
  c.ability.perma_mult = 2
  c.ability.forced_selection = true
  local h = MQOL.snapshot.build().hand[1]
  assert(h.enh == "m_glass" and h.ed == "e_holo" and h.seal == "Red")
  assert(h.perma.bonus == 5 and h.perma.mult == 2 and h.perma.x_mult == 1)
  assert(h.forced == true)
end

function tests.unenhanced_card_has_no_enh()
  G = fake.new()
  assert(MQOL.snapshot.build().hand[1].enh == nil)
end

function tests.face_down_card_masked()
  G = fake.new()
  local c = G.hand.cards[1]
  c.facing = "back"
  c.seal = "Red"
  c.edition = { key = "e_foil" }
  local h = MQOL.snapshot.build().hand[1]
  assert(h.hidden == true and h.id == 1)
  assert(h.rank == nil and h.suit == nil and h.enh == nil and h.ed == nil and h.seal == nil and h.perma == nil)
end

function tests.face_down_joker_masked()
  G = fake.new()
  G.jokers.cards[1] = fake.joker(9, "j_joker", { mult = 4 })
  G.jokers.cards[1].facing = "back"
  local j = MQOL.snapshot.build().jokers[1]
  assert(j.hidden == true and j.id == 9 and j.key == nil and j.ab == nil)
end

function tests.build_does_not_mutate_state()
  G = fake.new()
  G.jokers.cards[1] = fake.joker(9, "j_joker", { mult = 4 })
  local before = deep_copy(G)
  MQOL.snapshot.build()
  assert(deep_equal(before, G), "state changed")
end

function tests.functions_and_cycles_removed_from_ab()
  G = fake.new()
  local ability = { mult = 4, f = function() end, extra = { chips = 3 } }
  ability.self = ability
  G.jokers.cards[1] = fake.joker(9, "j_joker", ability)
  local ab = MQOL.snapshot.build().jokers[1].ab
  assert(ab.mult == 4 and ab.extra.chips == 3)
  assert(ab.f == nil and ab.self == nil)
  assert(MQOL.json.encode(ab))
end

function tests.unknown_mod_fields_pass_through_as_strings()
  G = fake.new()
  G.jokers.cards[1] = fake.joker(9, "j_modded_thing", { label = "weird \"text\"" })
  local c = G.hand.cards[1]
  c.config.center = { key = "m_modded", set = "Enhanced" }
  local s = MQOL.snapshot.build()
  assert(s.jokers[1].key == "j_modded_thing" and s.jokers[1].ab.label == "weird \"text\"")
  assert(s.hand[1].enh == "m_modded")
end

function tests.non_numeric_perma_values_do_not_raise()
  G = fake.new()
  local a = G.hand.cards[1].ability
  a.perma_x_mult = "weird"
  a.perma_bonus = {}
  a.perma_mult = true
  local h = MQOL.snapshot.build().hand[1]
  assert(h.perma.x_mult == 1 and h.perma.bonus == 0 and h.perma.mult == 0)
end

function tests.missing_optional_tables_do_not_raise()
  G = fake.new()
  G.GAME.current_round.idol_card = nil
  G.GAME.current_round.ancient_card = nil
  G.GAME.consumeable_usage_total = nil
  local s = MQOL.snapshot.build()
  assert(s.round.tarot_used == 0)
end

function tests.vouchers_and_consumables()
  G = fake.new()
  G.GAME.used_vouchers = { v_observatory = true, v_other = false }
  G.consumeables.cards[1] = { config = { center = { key = "c_pluto" } } }
  local s = MQOL.snapshot.build()
  assert(#s.vouchers == 1 and s.vouchers[1] == "v_observatory")
  assert(s.consumables[1].key == "c_pluto")
end

function tests.lobby_is_read_from_pvp()
  G = fake.new()
  PVP = { LOBBY = { code = "ABC" } }
  assert(MQOL.snapshot.build().lobby == true)
  PVP = { LOBBY = {} }
  assert(MQOL.snapshot.build().lobby == false)
  PVP = nil
end

function tests.selected_ids_follow_hand_position()
  G = fake.new()
  G.hand.highlighted = { G.hand.cards[3], G.hand.cards[1] }
  local ids = MQOL.snapshot.selected_ids()
  assert(#ids == 2 and ids[1] == 1 and ids[2] == 3)
end

function tests.json_round_trip_with_special_characters()
  local value = { a = 'q"uo\\te\n\t', b = { 1, 2.5, true }, c = {}, d = "é" }
  local out = MQOL.json.decode(MQOL.json.encode(value))
  assert(out.a == value.a and out.b[2] == 2.5 and out.b[3] == true and out.d == "é")
  assert(#out.c == 0)
end

function tests.json_decode_unicode_escape_and_null()
  local out = MQOL.json.decode('{"a":"\\u0041","b":null,"c":[1e3,-2]}')
  assert(out.a == "A" and out.b == nil and out.c[1] == 1000 and out.c[2] == -2)
end

function tests.json_rejects_garbage()
  assert(not pcall(MQOL.json.decode, "{nope"))
  assert(not pcall(MQOL.json.decode, "[1,]"))
end

function tests.json_encodes_non_finite_as_naneinf()
  assert(MQOL.json.encode({ math.huge }) == '["naneinf"]')
  assert(MQOL.json.encode({ -math.huge }) == '["naneinf"]')
  assert(MQOL.json.encode({ 0 / 0 }) == '["naneinf"]')
end

function tests.json_output_is_deterministic()
  local a = MQOL.json.encode({ z = 1, y = 2, x = { b = 1, a = 2 } })
  assert(a == '{"x":{"a":2,"b":1},"y":2,"z":1}', a)
end

function tests.json_large_integers_keep_precision()
  assert(MQOL.json.encode({ 123456789012 }) == "[123456789012]")
  assert(MQOL.json.decode(MQOL.json.encode({ 0.1 }))[1] == 0.1)
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
print("test_snapshot: ok (" .. #names .. ")")
