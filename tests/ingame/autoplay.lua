-- DEV ONLY. Never shipped: release packaging copies src/ and the entry point, not tests/.
-- Plays randomized hands on profile 3 so the live check can compare predictions with real
-- scores. .superpowers/devinstall.sh appends this to its copy of the entry point.
local KEYS = {
   "j_joker", "j_greedy_joker", "j_lusty_joker", "j_wrathful_joker", "j_gluttenous_joker",
   "j_jolly", "j_zany", "j_mad", "j_crazy", "j_droll", "j_sly", "j_wily", "j_clever",
   "j_devious", "j_crafty", "j_half", "j_stencil", "j_four_fingers", "j_mime", "j_credit_card",
   "j_ceremonial", "j_banner", "j_mystic_summit", "j_marble", "j_loyalty_card", "j_8_ball",
   "j_misprint", "j_dusk", "j_raised_fist", "j_chaos", "j_fibonacci", "j_steel_joker",
   "j_scary_face", "j_abstract", "j_delayed_grat", "j_hack", "j_pareidolia", "j_gros_michel",
   "j_even_steven", "j_odd_todd", "j_scholar", "j_business", "j_supernova", "j_ride_the_bus",
   "j_space", "j_egg", "j_burglar", "j_blackboard", "j_runner", "j_ice_cream", "j_dna",
   "j_splash", "j_blue_joker", "j_sixth_sense", "j_constellation", "j_hiker", "j_faceless",
   "j_green_joker", "j_superposition", "j_todo_list", "j_cavendish", "j_card_sharp",
   "j_red_card", "j_madness", "j_square", "j_seance", "j_riff_raff", "j_vampire", "j_shortcut",
   "j_hologram", "j_vagabond", "j_baron", "j_cloud_9", "j_rocket", "j_obelisk", "j_midas_mask",
   "j_luchador", "j_photograph", "j_gift", "j_turtle_bean", "j_erosion", "j_reserved_parking",
   "j_mail", "j_to_the_moon", "j_hallucination", "j_fortune_teller", "j_juggler", "j_drunkard",
   "j_stone", "j_golden", "j_lucky_cat", "j_baseball", "j_bull", "j_diet_cola", "j_trading",
   "j_flash", "j_popcorn", "j_trousers", "j_ancient", "j_ramen", "j_walkie_talkie", "j_selzer",
   "j_castle", "j_smiley", "j_campfire", "j_ticket", "j_mr_bones", "j_acrobat",
   "j_sock_and_buskin", "j_swashbuckler", "j_troubadour", "j_certificate", "j_smeared",
   "j_throwback", "j_hanging_chad", "j_rough_gem", "j_bloodstone", "j_arrowhead", "j_onyx_agate",
   "j_glass", "j_ring_master", "j_flower_pot", "j_blueprint", "j_wee", "j_merry_andy", "j_oops",
   "j_idol", "j_seeing_double", "j_matador", "j_hit_the_road", "j_duo", "j_trio", "j_family",
   "j_order", "j_tribe", "j_stuntman", "j_invisible", "j_brainstorm", "j_satellite",
   "j_shoot_the_moon", "j_drivers_license", "j_cartomancer", "j_astronomer", "j_burnt",
   "j_bootstraps", "j_caino", "j_triboulet", "j_yorick", "j_chicot", "j_perkeo",
}
local ENH = { "m_bonus", "m_mult", "m_wild", "m_glass", "m_steel", "m_stone", "m_gold", "m_lucky" }
local EDS = { { foil = true }, { holo = true }, { polychrome = true } }
local SEALS = { "Red", "Blue", "Gold", "Purple" }
local BLINDS = { "bl_flint", "bl_arm", "bl_psychic", "bl_eye", "bl_mouth", "bl_ox", "bl_head", "bl_club", "bl_goad", "bl_window", "bl_plant", "bl_tooth", "bl_small", "bl_small", "bl_small" }
-- With the file MortemQOL/scenarios present, these replace random setups, in a cycle. Each is a
-- combination that random deals rarely reach.
local SCENARIOS = {
  { name = "tooth_bull", blind = "bl_tooth", jokers = { "j_bull" }, cards = 5, dollars = 20 },
  { name = "tooth_bootstraps", blind = "bl_tooth", jokers = { "j_bootstraps" }, cards = 5, dollars = 12 },
  { name = "dna_hologram", jokers = { "j_dna", "j_hologram" }, cards = 1, first_hand = true },
  { name = "blueprint_dna_hologram", jokers = { "j_blueprint", "j_dna", "j_hologram" }, cards = 1, first_hand = true },
  { name = "dna_erosion", jokers = { "j_dna", "j_erosion" }, cards = 1, first_hand = true },
  { name = "loyalty_blueprint_fires", jokers = { "j_blueprint", "j_loyalty_card" }, cards = 1, loyalty = "fires" },
  { name = "loyalty_blueprint_quiet", jokers = { "j_blueprint", "j_loyalty_card" }, cards = 1, loyalty = "quiet" },
  { name = "money_tooth", blind = "bl_tooth", jokers = {}, cards = 5, dollars = 20 },
  { name = "money_ox", blind = "bl_ox", jokers = {}, cards = 1, dollars = 20, most_played = true },
  { name = "matador_blocked", blind = "bl_psychic", jokers = { "j_matador" }, cards = 1, dollars = 10 },
  { name = "blueprint_matador_blocked", blind = "bl_psychic", jokers = { "j_blueprint", "j_matador" }, cards = 1, dollars = 10 },
  { name = "gold_seal_retrigger", jokers = { "j_selzer" }, cards = 1, dollars = 10, gold_seal = true },
  { name = "todo_list", jokers = { "j_todo_list" }, cards = 1, dollars = 10, todo = true },
  { name = "rough_gem_bull", jokers = { "j_rough_gem", "j_bull" }, cards = 5, dollars = 10 },
  { name = "ox_bull", blind = "bl_ox", jokers = { "j_bull" }, cards = 1, dollars = 20, most_played = true },
}
local scenario_index = 0
local active = nil
local TARGET = 1e300
local WAIT_AFTER_SETUP = 0.6

local rng = love.math.newRandomGenerator(tonumber(os.getenv("MQOL_SEED") or "") or os.time())
local function pick(list) return list[rng:random(#list)] end

local started, phase, timer, stuck = false, "boot", 0, 0
local hands, seen = 0, {}

local function log(text)
  love.filesystem.createDirectory("MortemQOL")
  love.filesystem.append("MortemQOL/autoplay.log", text .. "\n")
end

local function start_run()
  G.FUNCS.start_run(nil, { stake = 1, seed = "MQOL" .. rng:random(1000, 9999) })
  phase, timer, stuck = "wait", 0, 0
end

local function randomize_card(card)
  if rng:random() < 0.35 then card:set_ability(G.P_CENTERS[pick(ENH)], nil, true) end
  if rng:random() < 0.2 then card:set_edition(pick(EDS), true, true) end
  if rng:random() < 0.2 then card:set_seal(pick(SEALS), true, true) end
  if rng:random() < 0.1 then card.ability.perma_bonus = rng:random(1, 20) end
end

local function randomize_joker(card)
  local a = card.ability
  if a.mult ~= nil and rng:random() < 0.7 then a.mult = rng:random(0, 12) end
  if rng:random() < 0.5 and a.x_mult and a.x_mult >= 1 and not a.type then a.x_mult = 1 + rng:random(0, 8) * 0.25 end
  if type(a.extra) == "table" and a.extra.chips ~= nil then a.extra.chips = rng:random(0, 60) end
  if rng:random() < 0.15 then card:set_edition(pick(EDS), true, true) end
end

local function rng_state()
  local keys = {}
  for k, v in pairs(G.GAME.pseudorandom) do keys[#keys + 1] = tostring(k) .. "=" .. tostring(v) end
  table.sort(keys)
  return table.concat(keys, ",")
end

local rng_before, rng_changes, worst_dt = nil, 0, 0

local function setup_scenario(sc)
  for i = #G.jokers.cards, 1, -1 do G.jokers.cards[i]:remove() end
  G.jokers.config.card_limit = 8
  for _, key in ipairs(sc.jokers) do SMODS.add_card({ key = key, set = "Joker", edition = "e_base" }) end
  G.GAME.dollars = sc.dollars or 10
  G.GAME.current_round.hands_left = 3
  for _, h in pairs(G.GAME.hands) do h.played_this_round = 0 end
  G.GAME.blind:set_blind(G.P_BLINDS[sc.blind or "bl_small"], nil, true)
  G.GAME.blind.chips = TARGET
  for _, card in ipairs(G.playing_cards) do G.GAME.blind:debuff_card(card) end
  if sc.first_hand then G.GAME.current_round.hands_played = 0 end
  if sc.gold_seal then G.hand.cards[1]:set_seal("Gold", true, true) end
  if sc.loyalty then
    for _, j in ipairs(G.jokers.cards) do
      if j.ability.name == "Loyalty Card" then
        j.ability.hands_played_at_create = 0
        G.GAME.hands_played = sc.loyalty == "fires" and 5 or 6
      end
    end
  end
  log("scenario " .. sc.name)
end

local function setup_hand()
  local wanted = love.filesystem.getInfo("MortemQOL/money") and "money" or "score"
  if MQOL.settings.mode() ~= wanted then MQOL.settings.set_mode(wanted) end
  if love.filesystem.getInfo("MortemQOL/scenarios") then
    scenario_index = scenario_index % #SCENARIOS + 1
    active = SCENARIOS[scenario_index]
    return setup_scenario(active)
  end
  active = nil
  for i = #G.jokers.cards, 1, -1 do G.jokers.cards[i]:remove() end
  G.jokers.config.card_limit = 8
  for _ = 1, rng:random(0, 6) do
    local key = pick(KEYS)
    local card = SMODS.add_card({ key = key, set = "Joker", edition = "e_base" })
    if card then
      seen[key] = (seen[key] or 0) + 1
      randomize_joker(card)
    end
  end
  for _, card in ipairs(G.hand.cards) do randomize_card(card) end
  G.GAME.dollars = rng:random(0, 40)
  G.GAME.current_round.discards_left = rng:random(0, 3)
  G.GAME.current_round.hands_left = rng:random() < 0.15 and 1 or rng:random(2, 4)
  -- A swapped blind starts fresh, so the round's per-hand history has to as well.
  for _, h in pairs(G.GAME.hands) do h.played_this_round = rng:random() < 0.05 and 1 or 0 end
  local key = pick(BLINDS)
  if key == "bl_eye" or key == "bl_mouth" then
    for _, h in pairs(G.GAME.hands) do h.played_this_round = 0 end
  end
  G.GAME.blind:set_blind(G.P_BLINDS[key], nil, true)
  G.GAME.blind.chips = TARGET
  for _, card in ipairs(G.playing_cards) do G.GAME.blind:debuff_card(card) end
end

local function choose_play()
  G.hand:unhighlight_all()
  local n = #G.hand.cards
  if active then
    for i = 1, math.min(active.cards, n) do G.hand:add_to_highlighted(G.hand.cards[i]) end
    if active.todo then
      local text = G.FUNCS.get_poker_hand_info(G.hand.highlighted)
      for _, j in ipairs(G.jokers.cards) do j.ability.to_do_poker_hand = text end
    end
    if active.most_played then
      local text = G.FUNCS.get_poker_hand_info(G.hand.highlighted)
      G.GAME.current_round.most_played_poker_hand = text
    end
    return
  end
  local state = MQOL.live.state()
  if rng:random() < 0.3 and state.result and MQOL.live.select_best() then return end
  local want = rng:random(1, math.min(5, n))
  local order = {}
  for i = 1, n do order[i] = i end
  for i = n, 2, -1 do
    local j = rng:random(i)
    order[i], order[j] = order[j], order[i]
  end
  for i = 1, want do G.hand:add_to_highlighted(G.hand.cards[order[i]]) end
end

-- Other mods keep the event queue busy, so "settled" means a second in the same state.
local last_state = nil
local function idle()
  return timer > 1
end

-- File switches for checking the gear without a mouse: MortemQOL/menu opens the menu, and
-- MortemQOL/assist_off turns assistance off.
local menu_opened = false
local function dev_switches()
  if love.filesystem.getInfo("MortemQOL/menu") and not menu_opened and G.STAGE == G.STAGES.RUN then
    menu_opened = true
    G.FUNCS.mqol_gear()
  end
  local assist = not love.filesystem.getInfo("MortemQOL/assist_off")
  if MQOL.settings.assist() ~= assist then MQOL.settings.set_assist(assist) end
end

local function step(dt)
  dev_switches()
  timer = timer + dt
  stuck = stuck + dt
  if G.STATE == G.STATES.SELECTING_HAND and dt > worst_dt then worst_dt = dt end
  if G.STATE ~= last_state then
    last_state = G.STATE
    timer = 0
  end
  if G.OVERLAY_MENU and phase ~= "boot" then
    G.FUNCS.exit_overlay_menu()
    return
  end
  if stuck > 60 then
    log("stuck in state " .. tostring(G.STATE) .. ", restarting")
    return start_run()
  end
  if phase == "boot" then
    if G.STATE == G.STATES.MENU and timer > 4 then
      G.SETTINGS.profile = 3
      G:load_profile(3)
      log("start")
      start_run()
    end
    return
  end
  if G.STATE == G.STATES.GAME_OVER then
    if timer > 1 then start_run() end
    return
  end
  if G.STATE == G.STATES.SHOP then
    if idle() then G.FUNCS.toggle_shop() ; timer, stuck = 0, 0 end
    return
  end
  if G.STATE == G.STATES.BLIND_SELECT then
    local opts = G.blind_select_opts
    if opts and idle() then
      local box = G.GAME.blind_on_deck and opts[string.lower(G.GAME.blind_on_deck)]
      local button = box and box:get_UIE_by_ID("select_blind_button")
      if button then G.FUNCS.select_blind(button); timer, stuck = 0, 0 end
    end
    return
  end
  if G.STATE ~= G.STATES.SELECTING_HAND or not idle() or #G.hand.cards == 0 then
    return
  end
  if phase ~= "setup_done" then
    if G.GAME.blind and G.GAME.blind.chips then setup_hand() end
    phase, timer = "setup_done", 0
    rng_before = rng_state()
    return
  end
  -- Creating this file freezes the script at hand selection, for screenshots.
  if love.filesystem.getInfo("MortemQOL/hold") then stuck = 0; return end
  if timer < WAIT_AFTER_SETUP or MQOL.live.state().status == "calculating" and timer < 5 then return end
  if rng_state() ~= rng_before then rng_changes = rng_changes + 1 end
  choose_play()
  G.FUNCS.play_cards_from_highlighted()
  hands = hands + 1
  phase, timer, stuck = "played", 0, 0
  if active then
    local c = MQOL.verify.counts()
    log(string.format("after %s match=%d mismatch=%d", active.name, c.match, c.mismatch))
  end
  if hands % 10 == 0 then
    local c = MQOL.verify.counts()
    local distinct = 0
    for _ in pairs(seen) do distinct = distinct + 1 end
    log(string.format("hands=%d match=%d mismatch=%d approximate=%d jokers_seen=%d rng_changes=%d worst_frame_ms=%.1f", hands, c.match, c.mismatch, c.approximate, distinct, rng_changes, worst_dt * 1000))
    worst_dt = 0
  end
end

local previous_update = love.update
function love.update(dt)
  previous_update(dt)
  local ok, err = pcall(step, dt)
  if not ok then log("autoplay error: " .. tostring(err)) end
  if G.SETTINGS then G.SETTINGS.GAMESPEED = 16 end
end
