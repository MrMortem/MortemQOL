-- The player's choices, kept in the mod's saved config.
local M = {}

local MODES = { score = true, money = true }
local config = nil
local owner = nil

function M._save()
  -- This takes the mod, not its config, and reports failure only through its return value.
  if owner then SMODS.save_mod_config(owner) end
end

function M.init(saved, mod)
  config = saved
  owner = mod
  if type(config.assist) ~= "boolean" then
    -- v1.0 called this show_panel.
    config.assist = config.show_panel ~= false
  end
  config.show_panel = nil
  if not MODES[config.mode] then config.mode = "score" end
end

local function save()
  pcall(M._save)
end

function M.assist()
  return config == nil or config.assist ~= false
end

function M.set_assist(on)
  if config == nil or type(on) ~= "boolean" then return end
  config.assist = on
  save()
end

function M.mode()
  return config and config.mode or "score"
end

function M.set_mode(mode)
  if config == nil or not MODES[mode] then return end
  config.mode = mode
  save()
end

-- What the engine is asked to do. Suggested discards and seeded mode aren't built yet.
function M.options()
  return { objective = M.mode(), discards = false, seeded = false }
end

return M
