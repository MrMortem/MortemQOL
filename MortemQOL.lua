-- MortemQOL entry point.
local mod = SMODS.current_mod

MQOL = MQOL or {}

local function load_module(name)
  return assert(SMODS.load_file("src/" .. name .. ".lua"))()
end

MQOL.settings = load_module("settings")
MQOL.settings.init(mod.config)
MQOL.json = load_module("json")
MQOL.bridge = load_module("bridge")
MQOL.snapshot = load_module("snapshot")
MQOL.live = load_module("live")
MQOL.format = load_module("format")
MQOL.panel_text = load_module("panel_text")
MQOL.verify = load_module("verify")
MQOL.ui = load_module("ui")
MQOL.ui.init(mod)

local ok, err = MQOL.bridge.load(mod.path)
if ok then
  sendInfoMessage("native engine loaded", "MortemQOL")
else
  sendWarnMessage("native engine unavailable: " .. tostring(err), "MortemQOL")
end

local ui_failed = false
local original_update = love.update
function love.update(dt)
  original_update(dt)
  MQOL.live.update(dt)
  local ok, ui_err = pcall(MQOL.ui.refresh)
  if not ok and not ui_failed then
    ui_failed = true
    sendWarnMessage("panel error: " .. tostring(ui_err), "MortemQOL")
  end
end

-- The live check only observes. The original functions always run exactly once, with the same
-- arguments and results.
local original_play = G.FUNCS.play_cards_from_highlighted
G.FUNCS.play_cards_from_highlighted = function(...)
  MQOL.verify.before_play()
  return original_play(...)
end

local original_evaluate = G.FUNCS.evaluate_play
G.FUNCS.evaluate_play = function(...)
  local results = { original_evaluate(...) }
  MQOL.verify.after_score(SMODS.last_hand_score)
  return unpack(results)
end
