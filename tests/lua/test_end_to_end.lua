-- Usage: luajit tests/lua/test_end_to_end.lua <dir containing the built library>
-- Runs the real snapshot, JSON, bridge and engine together on a fake game.
package.path = "./?.lua;./tests/lua/?.lua;" .. package.path
local ffi = require("ffi")
local fake = require("fake_game")
G = fake.new()
MQOL = {}
MQOL.settings = dofile("src/settings.lua")
MQOL.json = dofile("src/json.lua")
MQOL.snapshot = dofile("src/snapshot.lua")
MQOL.bridge = dofile("src/bridge.lua")
MQOL.live = dofile("src/live.lua")

local tmp = os.tmpname(); os.remove(tmp)
local sub = ffi.os == "OSX" and "macos" or "windows"
local name = ffi.os == "OSX" and "libmortemqol_engine.dylib" or "mortemqol_engine.dll"
assert(os.execute(('mkdir -p "%s/native/%s" && cp "%s/%s" "%s/native/%s/"'):format(tmp, sub, arg[1], name, tmp, sub)) == 0)
assert(MQOL.bridge.load(tmp))

G.GAME.hands["High Card"] = { level = 1, chips = 5, mult = 1, l_chips = 10, l_mult = 1, played = 0, played_this_round = 0, visible = true }
MQOL.live.update(1)
local deadline = os.clock() + 5
while MQOL.live.state().status == "calculating" and os.clock() < deadline do MQOL.live.update(0.01) end
local st = MQOL.live.state()
assert(st.status == "ready", "status " .. st.status)
assert(st.result.best.hand == "Pair", tostring(st.result.best.hand))
assert(st.result.best.expected == 64, tostring(st.result.best.expected))

G.hand.highlighted = { G.hand.cards[3] }
MQOL.live.update(1)
assert(MQOL.live.state().selection.expected == 8, "3 of clubs scores (5+3)*1")
assert(MQOL.live.select_best())
assert(#G.hand.highlighted == 2 and G.hand.highlighted[1].sort_id == 1)
os.execute(('rm -rf "%s"'):format(tmp))
print("test_end_to_end: ok")
