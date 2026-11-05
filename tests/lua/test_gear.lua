-- Usage: luajit tests/lua/test_gear.lua
-- Stubs the game's UI with the behavior gear.lua relies on.
local boxes

local function setup(room_x)
  boxes = {}
  G = {
    STAGES = { RUN = 1, MAIN_MENU = 2 },
    STAGE = 1,
    ROOM = { T = { x = room_x or 1.47, y = 1.1 } },
    UIT = { ROOT = 1, C = 2, R = 3, T = 4, O = 5 },
    C = { BLACK = {}, CLEAR = {}, UI = { TEXT_LIGHT = {} } },
    ASSET_ATLAS = {},
    FUNCS = {},
    HUD = {},
  }
  UIBox = function()
    local box = { T = { x = 0, y = 0, w = 1, h = 1 } }
    function box:remove() self.REMOVED = true end
    function box:hard_set_T(x, y, w, h) self.T = { x = x, y = y, w = w, h = h } end
    boxes[#boxes + 1] = box
    return box
  end
  create_toggle = function() return {} end
  create_option_cycle = function() return {} end
  SMODS = { Atlas = function() end }
  MQOL = { settings = dofile("src/settings.lua") }
  MQOL.settings._save = function() end
  MQOL.settings.init({})
  local gear = dofile("src/gear.lua")
  gear.init({ config = {} })
  return gear
end

local function live_boxes()
  local n = 0
  for _, box in ipairs(boxes) do
    if not box.REMOVED then n = n + 1 end
  end
  return n
end

local tests = {}

function tests.gear_appears_during_a_run()
  local gear = setup()
  gear.refresh()
  assert(live_boxes() == 1)
end

function tests.gear_returns_after_a_run_restart_removes_it()
  local gear = setup()
  gear.refresh()
  -- Game:delete_run removes stage objects, and the stage stays RUN through a restart.
  boxes[1]:remove()
  gear.refresh()
  assert(live_boxes() == 1, "gear did not come back")
end

function tests.gear_waits_for_the_new_run_to_be_set_up()
  -- delete_run and start_run are separate events with frames between them. A box made in that
  -- gap exists but is never drawn.
  local gear = setup()
  gear.refresh()
  boxes[1]:remove()
  G.HUD = nil
  gear.refresh()
  assert(live_boxes() == 0, "created during the restart gap")
  G.HUD = {}
  gear.refresh()
  assert(live_boxes() == 1, "should come back once the run is set up")
end

function tests.no_gear_before_the_first_hud()
  local gear = setup()
  G.HUD = nil
  gear.refresh()
  assert(live_boxes() == 0)
end

function tests.open_menu_returns_after_a_run_restart()
  local gear = setup()
  gear.refresh()
  G.FUNCS.mqol_gear()
  gear.refresh()
  assert(live_boxes() == 2)
  for _, box in ipairs(boxes) do box:remove() end
  gear.refresh()
  assert(live_boxes() == 2, "gear and menu should both return")
end

function tests.gear_goes_away_outside_a_run()
  local gear = setup()
  gear.refresh()
  G.STAGE = G.STAGES.MAIN_MENU
  gear.refresh()
  assert(live_boxes() == 0)
end

function tests.menu_toggles()
  local gear = setup()
  gear.refresh()
  G.FUNCS.mqol_gear()
  gear.refresh()
  assert(live_boxes() == 2)
  G.FUNCS.mqol_gear()
  gear.refresh()
  assert(live_boxes() == 1)
end

function tests.controls_stay_clickable_on_wide_windows()
  -- The cursor check gives up more than 2 units left of the room.
  local gear = setup(5.05)
  gear.refresh()
  G.FUNCS.mqol_gear()
  gear.refresh()
  assert(boxes[1].T.x >= -1.5, "gear x " .. boxes[1].T.x)
  assert(boxes[2].T.x >= -1.5, "menu x " .. boxes[2].T.x)
end

function tests.normal_window_keeps_the_corner_position()
  local gear = setup(1.0)
  gear.refresh()
  assert(math.abs(boxes[1].T.x - (0.35 - 1.0)) < 1e-9)
end

function tests.mode_callback_sets_the_mode()
  local gear = setup()
  gear.refresh()
  G.FUNCS.mqol_mode({ to_key = 2 })
  assert(MQOL.settings.mode() == "money")
  G.FUNCS.mqol_mode({ to_key = 1 })
  assert(MQOL.settings.mode() == "score")
end

local failed, names = 0, {}
for name in pairs(tests) do names[#names + 1] = name end
table.sort(names)
for _, name in ipairs(names) do
  local ok, err = pcall(tests[name])
  if not ok then failed = failed + 1; print("FAIL " .. name .. ": " .. tostring(err)) end
end
assert(failed == 0, failed .. " failed")
print("test_gear: ok (" .. #names .. ")")
