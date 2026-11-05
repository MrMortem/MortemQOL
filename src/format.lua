-- Display text for the panel. Numbers follow the game's number_format.
local M = {}

local E_SWITCH = 1e11

local function with_commas(n)
  local text = string.format("%d", n)
  local out = text:reverse():gsub("(%d%d%d)", "%1,"):reverse()
  return (out:gsub("^,", ""))
end

function M.score(x)
  if type(x) ~= "number" or x ~= x or x == math.huge or x == -math.huge then return "naneinf" end
  local sign = x < 0 and "-" or ""
  x = math.abs(x)
  if x >= E_SWITCH then
    local fac = math.floor(math.log10(x))
    local mantissa = tonumber(string.format("%.3f", x / 10 ^ fac))
    if mantissa >= 10 then
      mantissa = mantissa / 10
      fac = fac + 1
    end
    local pattern = fac >= 100 and "%.1fe%i" or fac >= 10 and "%.2fe%i" or "%.3fe%i"
    return sign .. string.format(pattern, mantissa, fac)
  end
  return sign .. with_commas(math.floor(x))
end

-- Text for a chance range, or nil when there is no range to show.
function M.range(floor, ceiling)
  if floor == ceiling then return nil end
  return M.score(floor) .. " - " .. M.score(ceiling)
end

function M.percent(x)
  if type(x) ~= "number" or x ~= x or x == math.huge then return "naneinf" end
  return string.format("%d%%", math.floor(x))
end

-- True for a number above zero, or the string form of an overflowed value.
function M.positive(x)
  return x == "naneinf" or (type(x) == "number" and x > 0)
end

local REASONS = {
  hidden_card = "A face-down card is in hand, so its effect is unknown.",
  hidden_joker = "A face-down joker's effect is unknown.",
  lobby = "Multiplayer rulesets can change scoring.",
  hook = "The Hook discards random cards before scoring.",
}

local PREFIXES = {
  { "unknown_joker:", "This joker isn't modeled: " },
  { "unknown_enhancement:", "This enhancement isn't modeled: " },
  { "unknown_edition:", "This edition isn't modeled: " },
  { "unknown_seal:", "This seal isn't modeled: " },
}

function M.reason(tag)
  if REASONS[tag] then return REASONS[tag] end
  for _, entry in ipairs(PREFIXES) do
    if tag:sub(1, #entry[1]) == entry[1] then return entry[2] .. tag:sub(#entry[1] + 1) end
  end
  return "Something here isn't modeled, so this is an estimate."
end

return M
