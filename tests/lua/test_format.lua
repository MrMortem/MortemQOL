-- Usage: luajit tests/lua/test_format.lua
local format = dofile("src/format.lua")

local function eq(a, b) assert(a == b, tostring(a) .. " ~= " .. tostring(b)) end

-- format_large_numbers_match_game_style
eq(format.score(0), "0")
eq(format.score(64), "64")
eq(format.score(1234), "1,234")
eq(format.score(1234567), "1,234,567")
eq(format.score(83.25), "83")
eq(format.score(1.23e15), "1.23e15")
eq(format.score(1.5e120), "1.5e120")
eq(format.score(1e11), "1.00e11")
eq(format.score(9.9996e15), "1.00e16")
-- format_naneinf_string
eq(format.score("naneinf"), "naneinf")
eq(format.score(math.huge), "naneinf")
eq(format.score(0 / 0), "naneinf")
-- range_nil_when_floor_equals_ceiling
eq(format.range(64, 64), nil)
eq(format.range(16, 336), "16 - 336")
eq(format.range("naneinf", "naneinf"), nil)
eq(format.range(16, "naneinf"), "16 - naneinf")
-- percent of the blind
eq(format.percent(21.33), "21%")
eq(format.percent(0), "0%")
eq(format.percent("naneinf"), "naneinf")
-- reason_for_each_approximate_tag
for _, tag in ipairs({ "hidden_card", "hidden_joker", "lobby", "hook", "unknown_joker:j_x", "unknown_enhancement:m_x", "unknown_edition:e_x", "unknown_seal:x" }) do
  local reason = format.reason(tag)
  assert(type(reason) == "string" and #reason > 0 and reason ~= format.reason("other_thing"), tag)
end
assert(format.reason("unknown_joker:j_x"):find("j_x", 1, true), "should name the joker")
assert(#format.reason("other_thing") > 0)
-- positive copes with the string form of huge numbers
assert(format.positive(5) == true)
assert(format.positive(0) == false)
assert(format.positive(nil) == false)
assert(format.positive("naneinf") == true)
-- money
eq(format.money(3), "+$3")
eq(format.money(4.5), "+$4.50")
eq(format.money(-5), "-$5")
eq(format.money(0), "$0")
eq(format.money(-1.25), "-$1.25")
eq(format.money("naneinf"), "naneinf")
eq(format.money(math.huge), "naneinf")
print("test_format: ok")
