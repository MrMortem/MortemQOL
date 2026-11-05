-- MortemQOL entry point.
local mod = SMODS.current_mod

MQOL = MQOL or {}

local function load_module(name)
  return assert(SMODS.load_file("src/" .. name .. ".lua"))()
end

MQOL.json = load_module("json")
MQOL.bridge = load_module("bridge")
MQOL.snapshot = load_module("snapshot")
MQOL.live = load_module("live")

local ok, err = MQOL.bridge.load(mod.path)
if ok then
  sendInfoMessage("native engine loaded", "MortemQOL")
else
  sendWarnMessage("native engine unavailable: " .. tostring(err), "MortemQOL")
end

local original_update = love.update
function love.update(dt)
  original_update(dt)
  MQOL.live.update(dt)
end
