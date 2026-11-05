-- Usage: luajit tests/lua/test_settings.lua
local function fresh(config)
  local settings = dofile("src/settings.lua")
  local saved = 0
  settings._save = function() saved = saved + 1 end
  settings.init(config)
  return settings, function() return saved end
end

local tests = {}

function tests.defaults_are_on_and_score()
  local s = fresh({})
  assert(s.assist() == true and s.mode() == "score")
end

function tests.legacy_show_panel_false_means_off()
  assert(fresh({ show_panel = false }).assist() == false)
  assert(fresh({ show_panel = true }).assist() == true)
end

function tests.assist_wins_over_legacy_show_panel()
  assert(fresh({ show_panel = false, assist = true }).assist() == true)
end

function tests.invalid_mode_falls_back_to_score()
  assert(fresh({ mode = "fame" }).mode() == "score")
  assert(fresh({ mode = 7 }).mode() == "score")
end

function tests.non_boolean_assist_falls_back_to_on()
  assert(fresh({ assist = "yes" }).assist() == true)
end

function tests.setters_update_and_save()
  local s, saved = fresh({})
  s.set_assist(false)
  assert(s.assist() == false and saved() == 1)
  s.set_mode("money")
  assert(s.mode() == "money" and saved() == 2)
end

function tests.set_mode_rejects_unknown()
  local s, saved = fresh({ mode = "money" })
  s.set_mode("fame")
  assert(s.mode() == "money" and saved() == 0)
end

function tests.options_report_objective_and_false_flags()
  local o = fresh({ mode = "money" }).options()
  assert(o.objective == "money" and o.discards == false and o.seeded == false)
end

function tests.works_before_init()
  local s = dofile("src/settings.lua")
  assert(s.assist() == true and s.mode() == "score")
end

function tests.save_failure_does_not_raise()
  local s = fresh({})
  s._save = function() error("disk full") end
  s.set_assist(false)
  assert(s.assist() == false)
end

local failed, names = 0, {}
for name in pairs(tests) do names[#names + 1] = name end
table.sort(names)
for _, name in ipairs(names) do
  local ok, err = pcall(tests[name])
  if not ok then failed = failed + 1; print("FAIL " .. name .. ": " .. tostring(err)) end
end
assert(failed == 0, failed .. " failed")
print("test_settings: ok (" .. #names .. ")")
