-- The calculator panel, joker and card badges, the Select button, and the config tab.
local M = {}

local PANEL_WIDTH = 4.2
local panel = nil
local was_dragging = false
local badge_signature = nil
local badged = {}
local reason_shown = nil

-- Text the panel's nodes read every frame.
local text = { hand = "", score = "", range = "", percent = "", selection = "", gain = "", status = "" }

local function line(key, scale, colour)
  return {
    n = G.UIT.R,
    config = { align = "cm", minh = 0.1 + scale },
    nodes = { { n = G.UIT.T, config = { ref_table = text, ref_value = key, scale = scale, colour = colour, shadow = true } } },
  }
end

local function panel_definition(reasons)
  local status_config = { align = "cm", minh = 0.28 }
  if reasons then
    status_config.tooltip = { title = "Approximate", text = reasons }
  end
  return {
    n = G.UIT.ROOT,
    config = { align = "cm", colour = G.C.BLACK, r = 0.1, padding = 0.12, emboss = 0.05, minw = PANEL_WIDTH },
    nodes = {
      {
        n = G.UIT.C,
        config = { align = "cm", minw = PANEL_WIDTH },
        nodes = {
          line("hand", 0.4, G.C.UI.TEXT_LIGHT),
          line("score", 0.55, G.C.GOLD),
          line("range", 0.3, G.C.UI.TEXT_LIGHT),
          line("percent", 0.3, G.C.UI.TEXT_LIGHT),
          line("selection", 0.3, G.C.UI.TEXT_LIGHT),
          line("gain", 0.3, G.C.GREEN),
          {
            n = G.UIT.R,
            config = status_config,
            nodes = { { n = G.UIT.T, config = { ref_table = text, ref_value = "status", scale = 0.3, colour = G.C.ORANGE, shadow = true } } },
          },
          {
            n = G.UIT.R,
            config = { align = "cm", padding = 0.05 },
            nodes = {
              {
                n = G.UIT.C,
                config = { align = "cm", button = "mqol_select", colour = G.C.RED, r = 0.1, padding = 0.08, minw = 1.4, hover = true, shadow = true },
                nodes = { { n = G.UIT.T, config = { text = "Select", scale = 0.35, colour = G.C.UI.TEXT_LIGHT, shadow = true } } },
              },
            },
          },
        },
      },
    },
  }
end

local function default_position()
  -- The empty space under the consumable slots.
  local area = G.consumeables and G.consumeables.T or { x = 15, y = 0.5, h = 3 }
  return area.x, area.y + area.h + 0.3
end

local function destroy_panel()
  if panel then panel:remove() end
  panel = nil
  reason_shown = nil
end

local function create_panel(reasons)
  destroy_panel()
  local config = MQOL.mod.config
  local x, y = default_position()
  panel = UIBox({
    definition = panel_definition(reasons),
    config = { align = "cm", offset = { x = 0, y = 0 } },
  })
  panel:hard_set_T(config.panel_x or x, config.panel_y or y, panel.T.w, panel.T.h)
  panel.states.drag.can = true
  panel.states.collide.can = true
end

local function clear_badges()
  for card, box in pairs(badged) do
    box:remove()
    if card.children then card.children.mqol_badge = nil end
  end
  badged = {}
  badge_signature = nil
end

local function add_badge(card, number, colour)
  local box = UIBox({
    definition = {
      n = G.UIT.ROOT,
      config = { align = "cm", colour = colour, r = 0.1, padding = 0.04, minw = 0.3 },
      nodes = { { n = G.UIT.T, config = { text = tostring(number), scale = 0.35, colour = G.C.UI.TEXT_LIGHT, shadow = true } } },
    },
    config = { align = "tm", major = card, bond = "Weak", offset = { x = 0, y = 0.05 }, parent = card },
  })
  box.states.collide.can = false
  card.children.mqol_badge = box
  badged[card] = box
end

local function by_id(cards, id)
  for _, card in ipairs(cards) do
    if card.sort_id == id then return card end
  end
  return nil
end

local function refresh_badges(result)
  local jokers = result and MQOL.format.positive(result.gain) and result.joker_order or nil
  local cards = result and result.card_order_matters and result.best and result.best.play or nil
  local signature = table.concat(jokers or {}, ",") .. "|" .. table.concat(cards or {}, ",")
  if signature == badge_signature then return end
  clear_badges()
  badge_signature = signature
  for position, id in ipairs(jokers or {}) do
    local card = by_id(G.jokers.cards, id)
    if card then add_badge(card, position, G.C.BLUE) end
  end
  for position, id in ipairs(cards or {}) do
    local card = by_id(G.hand.cards, id)
    if card then add_badge(card, position, G.C.PURPLE) end
  end
end

local function set_text(state)
  local format = MQOL.format
  for key in pairs(text) do text[key] = "" end
  local result = state.result
  local best = result and result.best
  if state.status == "unavailable" then
    text.status = "Engine unavailable"
    return
  end
  if best then
    text.hand = best.hand
    text.score = format.score(best.expected)
    local range = format.range(best.floor, best.ceiling)
    text.range = range and ("Range " .. range) or ""
    text.percent = format.percent(best.blind_pct) .. " of blind"
  elseif state.status == "ready" then
    text.hand = "No scoring play"
  end
  if state.selection then
    text.selection = "Selected: " .. format.score(state.selection.expected)
  end
  if result and format.positive(result.gain) then
    local label = result.order_exact and "Reorder jokers: +" or "Reorder jokers (best found): +"
    text.gain = label .. format.score(result.gain)
  end
  local mismatches = MQOL.verify and MQOL.verify.counts().mismatch or 0
  if state.status == "calculating" then
    text.status = "Calculating..."
  elseif mismatches > 0 then
    text.status = mismatches .. " score mismatch" .. (mismatches == 1 and "" or "es") .. " logged"
  elseif result and result.approximate and #result.approximate > 0 then
    text.status = "Approximate (hover)"
  end
end

local function reasons_for(result)
  local tags = result and result.approximate
  if not tags or #tags == 0 then return nil end
  local out = {}
  for i, tag in ipairs(tags) do out[i] = MQOL.format.reason(tag) end
  return out
end

local function remember_position()
  local dragging = panel.states.drag.is
  if was_dragging and not dragging then
    MQOL.mod.config.panel_x = panel.T.x
    MQOL.mod.config.panel_y = panel.T.y
    if SMODS.save_mod_config then SMODS.save_mod_config(MQOL.mod) end
  end
  was_dragging = dragging
end

function M.refresh()
  local state = MQOL.live.state()
  local enabled = MQOL.mod.config.show_panel
  local selecting = G.STAGE == G.STAGES.RUN and G.STATE == G.STATES.SELECTING_HAND
  if not enabled or not selecting or state.status == "idle" then
    destroy_panel()
    clear_badges()
    return
  end
  local reasons = reasons_for(state.result)
  local reason_key = reasons and table.concat(reasons, "|") or ""
  if not panel or reason_key ~= reason_shown then
    create_panel(reasons)
    reason_shown = reason_key
  end
  set_text(state)
  refresh_badges(state.result)
  remember_position()
end

function M.init(mod)
  MQOL.mod = mod
  G.FUNCS.mqol_select = function()
    MQOL.live.select_best()
  end
  mod.config_tab = function()
    return {
      n = G.UIT.ROOT,
      config = { r = 0.1, minw = 6, align = "cm", padding = 0.2, colour = G.C.BLACK },
      nodes = {
        create_toggle({ label = "Show calculator panel", ref_table = mod.config, ref_value = "show_panel" }),
      },
    }
  end
end

return M
