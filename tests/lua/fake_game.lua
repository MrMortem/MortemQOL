-- A minimal stand-in for the game's globals.
local M = {}

function M.card(id, rank, suit, extra)
  local c = {
    sort_id = id,
    base = { id = rank, suit = suit },
    config = { center = { key = "c_base", set = "Default" } },
    ability = { set = "Default" },
    facing = "front",
    debuff = false,
  }
  for k, v in pairs(extra or {}) do c[k] = v end
  return c
end

function M.joker(id, key, ability)
  return {
    sort_id = id,
    config = { center = { key = key, rarity = 1, set = "Joker" } },
    ability = ability or {},
    facing = "front",
    debuff = false,
    sell_cost = 2,
  }
end

function M.new()
  local deck_cards = {}
  local suits = { "Spades", "Hearts", "Clubs", "Diamonds" }
  for i = 1, 44 do deck_cards[i] = M.card(100 + i, (i % 13) + 2, suits[(i % 4) + 1]) end
  local playing = {}
  for i = 1, 52 do playing[i] = true end
  local g = {
    STATES = { SELECTING_HAND = 1, HAND_PLAYED = 2, DRAW_TO_HAND = 3, NEW_ROUND = 4 },
    STAGES = { RUN = 1, MAIN_MENU = 2 },
    STAGE = 1,
    STATE = 1,
    deck = { cards = deck_cards },
    playing_cards = playing,
    hand = { cards = { M.card(1, 14, "Spades"), M.card(2, 14, "Hearts"), M.card(3, 3, "Clubs") }, highlighted = {} },
    jokers = { cards = {} },
    consumeables = { cards = {} },
    GAME = {
      hands = { Pair = { level = 1, chips = 10, mult = 2, l_chips = 15, l_mult = 1, played = 0, played_this_round = 0, visible = true } },
      blind = { config = { blind = { key = "bl_small" } }, disabled = false, chips = 300 },
      current_round = {
        hands_left = 4, discards_left = 3, hands_played = 0,
        idol_card = { id = 14, suit = "Spades" },
        ancient_card = { suit = "Hearts" },
        most_played_poker_hand = "High Card",
      },
      dollars = 4, chips = 0, hands_played = 0,
      starting_deck_size = 52,
      selected_back = { effect = { center = { key = "b_red" } } },
      used_vouchers = {},
      consumeable_usage_total = { tarot = 2 },
    },
  }
  function g.hand:unhighlight_all() self.highlighted = {} end
  function g.hand:add_to_highlighted(card) self.highlighted[#self.highlighted + 1] = card end
  return g
end

return M
