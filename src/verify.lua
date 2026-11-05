-- Compares each real play with its prediction and logs anything that disagrees.
local M = {}

local TOLERANCE = 1e-9
local DIR = "MortemQOL"

local counts = { match = 0, mismatch = 0, approximate = 0 }
local pending = nil
-- A scored play waiting for its money to settle, which happens after the game's delayed events.
local money_pending = nil

function M._write(file, text)
  love.filesystem.createDirectory(DIR)
  love.filesystem.append(DIR .. "/" .. file, text)
end

local function is_infinite(x)
  return x == "naneinf" or (type(x) == "number" and (x ~= x or x == math.huge or x == -math.huge))
end

local function compare_values(prediction, actual)
  if prediction.expected == "naneinf" then
    return is_infinite(actual) and "match" or "mismatch"
  end
  if is_infinite(actual) then return "mismatch" end
  if prediction.legal == false then
    return actual == 0 and "match" or "mismatch"
  end
  if prediction.has_chance then
    local ceiling = prediction.ceiling == "naneinf" and math.huge or prediction.ceiling
    local slack = TOLERANCE * math.max(math.abs(ceiling == math.huge and actual or ceiling), 1)
    return (actual >= prediction.floor - slack and actual <= ceiling + slack) and "match" or "mismatch"
  end
  local slack = TOLERANCE * math.max(math.abs(prediction.expected), 1)
  return math.abs(actual - prediction.expected) <= slack and "match" or "mismatch"
end

function M.compare(prediction, actual)
  if prediction.approximate and #prediction.approximate > 0 then return "approximate" end
  return compare_values(prediction, actual)
end

local function compare_money_values(prediction, actual)
  local floor, ceiling = prediction.money_floor, prediction.money_ceiling
  if type(floor) ~= "number" or type(ceiling) ~= "number" then return "match" end
  local slack = TOLERANCE * math.max(math.abs(floor), math.abs(ceiling), 1)
  return (actual >= floor - slack and actual <= ceiling + slack) and "match" or "mismatch"
end

function M.compare_money(prediction, actual)
  if prediction.approximate and #prediction.approximate > 0 then return "approximate" end
  return compare_money_values(prediction, actual)
end

function M.counts()
  return counts
end

-- Call just before the game plays the highlighted cards.
function M.before_play()
  pending = nil
  money_pending = nil
  pcall(function()
    if not MQOL.settings.assist() or not MQOL.bridge.available() then return end
    local snapshot = MQOL.snapshot.build()
    local play = MQOL.snapshot.selected_ids()
    if #play == 0 then return end
    local status, json = MQOL.bridge.score(MQOL.json.encode({ snapshot = snapshot, play = play }))
    if status == 1 and json then
      pending = {
        snapshot = snapshot,
        play = play,
        prediction = MQOL.json.decode(json),
        balance = G.GAME.dollars,
      }
    end
  end)
end

-- Call with the game's final score for the play.
function M.after_score(actual)
  local entry = pending
  pending = nil
  if not entry then return end
  money_pending = entry
  pcall(function()
    local outcome = M.compare(entry.prediction, actual)
    counts[outcome] = counts[outcome] + 1
    local wrong = outcome == "mismatch"
      or (outcome == "approximate" and compare_values(entry.prediction, actual) == "mismatch")
    if wrong then
      local line = { snapshot = entry.snapshot, play = entry.play, prediction = entry.prediction, actual = actual }
      local file = outcome == "mismatch" and "verify.jsonl" or "verify_approximate.jsonl"
      M._write(file, MQOL.json.encode(line) .. "\n")
    end
  end)
end

-- Call every frame. Compares money when the hand finishes normally, which is after its money
-- events and before end-of-round payouts. Any other way out, such as quitting mid-hand, drops
-- unpaid money and can replace the game, so that comparison is discarded.
function M.tick()
  local entry = money_pending
  if not entry or G.STATE == G.STATES.HAND_PLAYED then return end
  money_pending = nil
  local finished = G.STAGE == G.STAGES.RUN
    and (G.STATE == G.STATES.DRAW_TO_HAND or G.STATE == G.STATES.NEW_ROUND)
  if not finished then return end
  pcall(function()
    local actual = G.GAME.dollars - entry.balance
    local outcome = M.compare_money(entry.prediction, actual)
    counts[outcome] = counts[outcome] + 1
    local wrong = outcome == "mismatch"
      or (outcome == "approximate" and compare_money_values(entry.prediction, actual) == "mismatch")
    if wrong then
      local line = { snapshot = entry.snapshot, play = entry.play, prediction = entry.prediction, actual_money = actual }
      local file = outcome == "mismatch" and "verify.jsonl" or "verify_approximate.jsonl"
      M._write(file, MQOL.json.encode(line) .. "\n")
    end
  end)
end

return M
