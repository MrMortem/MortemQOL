-- The gear icon at the top left and its menu.
local M = {}

local ATLAS = "MortemQOL_gear"
-- From the window's top-left corner, above the blind info.
local GEAR_X, GEAR_Y = 0.35, 0.25

local gear = nil
local menu = nil
local menu_open = false
local menu_values = nil

-- Backing values for the game's toggle and option cycle.
local form = { assist = true }
local MODE_LABELS = { "Score", "Money" }
local MODE_KEYS = { "score", "money" }

local function mode_index()
  return MQOL.settings.mode() == "money" and 2 or 1
end

local function controls()
  form.assist = MQOL.settings.assist()
  return {
    create_toggle({ label = "Assistance", ref_table = form, ref_value = "assist", callback = MQOL.settings.set_assist }),
    create_option_cycle({
      label = "Mode",
      options = MODE_LABELS,
      current_option = mode_index(),
      opt_callback = "mqol_mode",
      scale = 0.8,
      w = 3,
    }),
  }
end

local function destroy_menu()
  if menu then menu:remove() end
  menu = nil
end

-- What the open menu is showing, so a change made elsewhere can refresh it.
local function current_values()
  return tostring(MQOL.settings.assist()) .. MQOL.settings.mode()
end

local function open_menu()
  destroy_menu()
  menu_values = current_values()
  menu = UIBox({
    definition = {
      n = G.UIT.ROOT,
      config = { align = "cm", colour = G.C.BLACK, r = 0.1, padding = 0.1, emboss = 0.05 },
      nodes = { { n = G.UIT.C, config = { align = "cm", padding = 0.05 }, nodes = controls() } },
    },
    config = { align = "cm", offset = { x = 0, y = 0 } },
  })
  menu:hard_set_T(GEAR_X - G.ROOM.T.x, GEAR_Y - G.ROOM.T.y + 0.75, menu.T.w, menu.T.h)
end

local function gear_face()
  local atlas = G.ASSET_ATLAS and G.ASSET_ATLAS[ATLAS]
  if atlas then
    local sprite = Sprite(0, 0, 0.55, 0.55, atlas, { x = 0, y = 0 })
    return { n = G.UIT.O, config = { object = sprite } }
  end
  return { n = G.UIT.T, config = { text = "QOL", scale = 0.3, colour = G.C.UI.TEXT_LIGHT, shadow = true } }
end

local function create_gear()
  gear = UIBox({
    definition = {
      n = G.UIT.ROOT,
      config = { align = "cm", colour = G.C.CLEAR },
      nodes = {
        {
          n = G.UIT.C,
          config = { align = "cm", button = "mqol_gear", colour = G.C.BLACK, r = 0.1, padding = 0.04, hover = true, shadow = true },
          nodes = { gear_face() },
        },
      },
    },
    config = { align = "cm", offset = { x = 0, y = 0 } },
  })
  gear:hard_set_T(GEAR_X - G.ROOM.T.x, GEAR_Y - G.ROOM.T.y, gear.T.w, gear.T.h)
end

local function destroy_all()
  destroy_menu()
  if gear then gear:remove() end
  gear = nil
  menu_open = false
end

function M.refresh()
  if G.STAGE ~= G.STAGES.RUN then
    if gear then destroy_all() end
    return
  end
  if not gear then create_gear() end
  if menu_open and (not menu or menu_values ~= current_values()) then open_menu() end
  if not menu_open and menu then destroy_menu() end
end

function M.init(mod)
  G.FUNCS.mqol_gear = function()
    menu_open = not menu_open
  end
  G.FUNCS.mqol_mode = function(args)
    MQOL.settings.set_mode(MODE_KEYS[args.to_key])
  end
  SMODS.Atlas({ key = "gear", path = "gear.png", px = 34, py = 34 })
  mod.config_tab = function()
    return {
      n = G.UIT.ROOT,
      config = { r = 0.1, minw = 6, align = "cm", padding = 0.2, colour = G.C.BLACK },
      nodes = { { n = G.UIT.C, config = { align = "cm", padding = 0.1 }, nodes = controls() } },
    }
  end
end

return M
