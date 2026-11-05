-- Turns the live state into the panel's lines. Every field is a string, empty when unused.
local M = {}

local function blank()
  return { hand = "", score = "", range = "", percent = "", selection = "", gain = "", status = "", money = "", warning = "" }
end

local function money_line(format, best)
  local line = format.money(best.money_expected)
  if best.money_floor ~= best.money_ceiling then
    line = line .. " (" .. format.money(best.money_floor) .. " to " .. format.money(best.money_ceiling) .. ")"
  end
  return line
end

local function gain_line(format, result, mode)
  local label = result.order_exact == false and "Reorder jokers (best found): +" or "Reorder jokers: +"
  if mode == "money" and format.positive(result.gain_money) then
    local money = format.money(result.gain_money)
    return label .. (money:gsub("^%+", ""))
  end
  if format.positive(result.gain) then return label .. format.score(result.gain) end
  return ""
end

local function status_line(state, mismatches)
  local parts = {}
  if state.status == "calculating" then parts[#parts + 1] = "Calculating..." end
  if mismatches > 0 then
    parts[#parts + 1] = mismatches .. " score mismatch" .. (mismatches == 1 and "" or "es") .. " logged"
  end
  local tags = state.result and state.result.approximate
  if tags and #tags > 0 then parts[#parts + 1] = "Approximate (hover)" end
  return table.concat(parts, " | ")
end

function M.build(state, mode, mismatches)
  local format = MQOL.format
  local text = blank()
  if state.status == "unavailable" then
    text.status = "Engine unavailable"
    return text
  end
  local result = state.result
  local best = result and result.best
  if best then
    text.hand = result.blocked and "Blocked: scores 0" or best.hand
    text.score = format.score(best.expected)
    local range = format.range(best.floor, best.ceiling)
    text.range = range and ("Range " .. range) or ""
    text.percent = format.percent(best.blind_pct) .. " of blind"
    if mode == "money" then
      text.money = money_line(format, best)
      if result.blind_warning then text.warning = "Won't beat the blind" end
    end
  elseif state.status == "ready" then
    text.hand = "No scoring play"
  end
  if state.selection then
    local score = format.score(state.selection.expected)
    if mode == "money" then
      text.selection = "Selected: " .. format.money(state.selection.money_expected) .. ", " .. score
    else
      text.selection = "Selected: " .. score
    end
  end
  if result then text.gain = gain_line(format, result, mode) end
  text.status = status_line(state, mismatches)
  return text
end

return M
