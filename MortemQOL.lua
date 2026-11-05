-- MortemQOL entry point.
local mod = SMODS.current_mod

MQOL = MQOL or {}

local function load_module(name)
  return assert(SMODS.load_file("src/" .. name .. ".lua"))()
end

MQOL.bridge = load_module("bridge")

local ok, err = MQOL.bridge.load(mod.path)
if ok then
  sendInfoMessage("native engine loaded", "MortemQOL")
else
  sendWarnMessage("native engine unavailable: " .. tostring(err), "MortemQOL")
end
