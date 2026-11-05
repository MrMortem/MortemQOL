-- Reads what the player can see into plain tables. Writes nothing to the game, never reads
-- deck order, and masks anything face down.
local M = {}

local SCHEMA = 1
local MAX_DEPTH = 5

-- Copies ability values, dropping functions, userdata and cycles.
local function sanitize(value, seen, depth)
  local kind = type(value)
  if kind == "number" or kind == "string" or kind == "boolean" then return value end
  if kind ~= "table" or seen[value] or depth > MAX_DEPTH then return nil end
  seen[value] = true
  local out = {}
  for k, v in pairs(value) do
    if type(k) == "string" or type(k) == "number" then
      local clean = sanitize(v, seen, depth + 1)
      if clean ~= nil then out[k] = clean end
    end
  end
  seen[value] = nil
  return out
end

-- Other mods can put anything in these fields.
local function number_or(value, default)
  return type(value) == "number" and value or default
end

local function edition_key(card)
  return card.edition and card.edition.key or nil
end

local function hand_card(card)
  if card.facing == "back" then
    return { id = card.sort_id, hidden = true }
  end
  local center = card.config and card.config.center
  local ability = card.ability or {}
  local perma_x = number_or(ability.perma_x_mult, 0)
  return {
    id = card.sort_id,
    rank = card.base and card.base.id or 0,
    suit = card.base and card.base.suit or "",
    enh = center and center.set == "Enhanced" and center.key or nil,
    ed = edition_key(card),
    seal = card.seal,
    perma = {
      bonus = number_or(ability.perma_bonus, 0),
      mult = number_or(ability.perma_mult, 0),
      x_mult = perma_x > 0 and perma_x or 1,
    },
    debuff = card.debuff and true or false,
    forced = ability.forced_selection and true or false,
    hidden = false,
  }
end

local function joker_card(card)
  if card.facing == "back" then
    return { id = card.sort_id, hidden = true }
  end
  local center = card.config and card.config.center or {}
  return {
    id = card.sort_id,
    key = center.key,
    rarity = center.rarity,
    ed = edition_key(card),
    debuff = card.debuff and true or false,
    hidden = false,
    sell = card.sell_cost or 0,
    ab = sanitize(card.ability or {}, {}, 1),
  }
end

local function map(cards, fn)
  local out = {}
  for i, card in ipairs(cards or {}) do out[i] = fn(card) end
  return out
end

local function levels()
  local out = {}
  for name, h in pairs(G.GAME.hands or {}) do
    out[name] = {
      level = h.level,
      chips = h.chips,
      mult = h.mult,
      l_chips = h.l_chips,
      l_mult = h.l_mult,
      played = h.played or 0,
      played_round = h.played_this_round or 0,
      visible = h.visible and true or false,
    }
  end
  return out
end

local function vouchers()
  local out = {}
  for key, used in pairs(G.GAME.used_vouchers or {}) do
    if used then out[#out + 1] = key end
  end
  table.sort(out)
  return out
end

local function sort_text(value)
  return value or ""
end

-- Counts the remaining deck into groups, which is what the game's deck view shows. Grouping and
-- sorting mean no order survives. Nothing else may read individual deck cards.
local function deck_contents()
  local groups, by_key = {}, {}
  for _, card in ipairs(G.deck and G.deck.cards or {}) do
    local center = card.config and card.config.center
    local rank = number_or(card.base and card.base.id, 0)
    local suit = card.base and card.base.suit or ""
    local enh = center and center.set == "Enhanced" and center.key or nil
    local ed = edition_key(card)
    local seal = card.seal
    local key = table.concat({ rank, suit, sort_text(enh), sort_text(ed), sort_text(seal) }, "|")
    local group = by_key[key]
    if not group then
      group = { rank = rank, suit = suit, enh = enh, ed = ed, seal = seal, count = 0 }
      by_key[key] = group
      groups[#groups + 1] = group
    end
    group.count = group.count + 1
  end
  table.sort(groups, function(a, b)
    if a.rank ~= b.rank then return a.rank < b.rank end
    if a.suit ~= b.suit then return a.suit < b.suit end
    if sort_text(a.enh) ~= sort_text(b.enh) then return sort_text(a.enh) < sort_text(b.enh) end
    if sort_text(a.ed) ~= sort_text(b.ed) then return sort_text(a.ed) < sort_text(b.ed) end
    return sort_text(a.seal) < sort_text(b.seal)
  end)
  return groups
end

local function in_lobby()
  return (PVP and PVP.LOBBY and PVP.LOBBY.code ~= nil) and true or false
end

function M.build()
  local game = G.GAME
  local round = game.current_round or {}
  local idol = round.idol_card or {}
  local ancient = round.ancient_card or {}
  local usage = game.consumeable_usage_total or {}
  local blind = game.blind or {}
  local blind_config = blind.config and blind.config.blind or {}
  local back = game.selected_back and game.selected_back.effect and game.selected_back.effect.center or {}
  return {
    schema = SCHEMA,
    hand = map(G.hand and G.hand.cards, hand_card),
    jokers = map(G.jokers and G.jokers.cards, joker_card),
    levels = levels(),
    blind = {
      key = blind_config.key or "",
      disabled = blind.disabled and true or false,
      target = blind.chips or 0,
    },
    round = {
      hands_left = round.hands_left or 0,
      discards_left = round.discards_left or 0,
      dollars = game.dollars or 0,
      chips = game.chips or 0,
      idol = { rank = idol.id or 0, suit = idol.suit or "" },
      ancient_suit = ancient.suit or "",
      hands_played = game.hands_played or 0,
      round_hands = round.hands_played or 0,
      tarot_used = usage.tarot or 0,
      most_played = round.most_played_poker_hand or "",
    },
    deck = {
      back = back.key or "",
      -- Lengths only. These counts are visible to the player, and no card is read.
      remaining = G.deck and #G.deck.cards or 0,
      total = G.playing_cards and #G.playing_cards or 0,
      starting = game.starting_deck_size or 0,
      contents = deck_contents(),
    },
    vouchers = vouchers(),
    consumables = map(G.consumeables and G.consumeables.cards, function(card)
      return { key = card.config and card.config.center and card.config.center.key or "" }
    end),
    lobby = in_lobby(),
    options = MQOL.settings.options(),
  }
end

-- Highlighted card ids, in the order they sit in hand, which is the order they would play.
function M.selected_ids()
  local chosen = {}
  for _, card in ipairs(G.hand and G.hand.highlighted or {}) do chosen[card] = true end
  local ids = {}
  for _, card in ipairs(G.hand and G.hand.cards or {}) do
    if chosen[card] then ids[#ids + 1] = card.sort_id end
  end
  return ids
end

return M
