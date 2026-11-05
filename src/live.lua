-- Keeps a search running for the current game state. Mod errors never reach the game.
local M = {}

local REBUILD_INTERVAL = 0.1

local state

function M._reset()
  state = {
    status = "idle",
    result = nil,
    selection = nil,
    search_id = nil,
    snapshot_json = nil,
    snapshot = nil,
    selection_key = nil,
    timer = REBUILD_INTERVAL,
    logged = false,
    mode = nil,
  }
end

M._reset()

local function log_once(message)
  if state.logged then return end
  state.logged = true
  if sendWarnMessage then sendWarnMessage(tostring(message), "MortemQOL") end
end

local function selecting_hand()
  return G and G.STATES and G.STATE == G.STATES.SELECTING_HAND and G.hand ~= nil
end

local function clear()
  state.status = "idle"
  state.result = nil
  state.selection = nil
  state.search_id = nil
  state.snapshot_json = nil
  state.snapshot = nil
  state.selection_key = nil
end

local function refresh_search()
  local snapshot = MQOL.snapshot.build()
  local json = MQOL.json.encode(snapshot)
  state.snapshot = snapshot
  if json == state.snapshot_json then return end
  state.snapshot_json = json
  state.result = nil
  state.selection_key = nil
  state.search_id = MQOL.bridge.search_start(json)
  state.status = state.search_id and "calculating" or "unavailable"
end

local function refresh_selection()
  local ids = MQOL.snapshot.selected_ids()
  local key = table.concat(ids, ",")
  if key == state.selection_key then return end
  state.selection_key = key
  state.selection = nil
  if #ids == 0 then return end
  local request = MQOL.json.encode({ snapshot = state.snapshot, play = ids })
  local status, json = MQOL.bridge.score(request)
  if status == 1 and json then state.selection = MQOL.json.decode(json) end
end

local function poll()
  if not state.search_id then return end
  local status, json = MQOL.bridge.search_poll(state.search_id)
  if status == 0 then
    if json then state.result = MQOL.json.decode(json) end
  elseif status == 1 then
    if json then state.result = MQOL.json.decode(json) end
    state.status = "ready"
    state.search_id = nil
  elseif status == -1 then
    log_once(json or "engine error")
    state.result = nil
    state.status = "unavailable"
    state.search_id = nil
  else
    state.search_id = nil
  end
end

local function step(dt)
  if not MQOL.settings.assist() then
    if state.status ~= "idle" then clear() end
    state.timer = REBUILD_INTERVAL
    return
  end
  if state.mode ~= MQOL.settings.mode() then
    -- Drop the old mode's result now, rather than at the next rebuild.
    state.mode = MQOL.settings.mode()
    clear()
    state.timer = REBUILD_INTERVAL
  end
  if not MQOL.bridge.available() then
    state.status = "unavailable"
    return
  end
  if not selecting_hand() then
    if state.status ~= "idle" then clear() end
    state.timer = REBUILD_INTERVAL
    return
  end
  state.timer = state.timer + dt
  if state.timer >= REBUILD_INTERVAL then
    state.timer = 0
    refresh_search()
    refresh_selection()
  end
  poll()
end

function M.update(dt)
  local ok, err = pcall(step, dt)
  if not ok then
    log_once(err)
    state.status = "unavailable"
    state.result = nil
    state.search_id = nil
    state.snapshot_json = nil
  end
end

function M.state()
  return state
end

-- Highlights the recommended cards. Returns false and changes nothing if they cannot all be
-- found. Never plays a hand.
function M.select_best()
  local best = state.result and state.result.best
  if not best or not selecting_hand() then return false end
  local cards = {}
  for i, id in ipairs(best.play) do
    for _, card in ipairs(G.hand.cards) do
      if card.sort_id == id then cards[i] = card end
    end
    if not cards[i] then return false end
  end
  G.hand:unhighlight_all()
  local already = {}
  for _, card in ipairs(G.hand.highlighted) do already[card] = true end
  for _, card in ipairs(cards) do
    if not already[card] then G.hand:add_to_highlighted(card) end
  end
  return true
end

return M
