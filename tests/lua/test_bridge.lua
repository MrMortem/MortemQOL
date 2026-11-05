-- Usage: luajit tests/lua/test_bridge.lua <dir containing the built library>
package.path = "./?.lua;" .. package.path
local ffi = require("ffi")
local bridge = dofile("src/bridge.lua")

-- missing path fails softly
local ok, err = bridge.load("/nonexistent/mod/path")
assert(ok == false and type(err) == "string", "missing library should return false and a message")
assert(bridge.available() == false, "should be unavailable after a failed load")

-- wrong ABI version is a load failure
local real_loader = bridge._loader
bridge._loader = function() return { mqol_abi_version = function() return 2 end } end
ok, err = bridge.load("/any")
assert(ok == false and err:find("ABI"), "wrong ABI should fail: " .. tostring(err))
assert(bridge.available() == false)
bridge._loader = real_loader

-- the real library loads from the mod's native folder
local tmp = os.tmpname(); os.remove(tmp)
local sub = ffi.os == "OSX" and "macos" or "windows"
local name = ffi.os == "OSX" and "libmortemqol_engine.dylib" or "mortemqol_engine.dll"
assert(os.execute(('mkdir -p "%s/native/%s" && cp "%s/%s" "%s/native/%s/"'):format(tmp, sub, arg[1], name, tmp, sub)) == 0)
ok, err = bridge.load(tmp .. "/")
assert(ok == true, "real library should load: " .. tostring(err))
assert(bridge.available() == true)

-- a too-small buffer is retried internally, and errors come back as a status
local status, msg = bridge.score("{}")
assert(status == -1 and type(msg) == "string" and #msg > 0, "score should report an error message")
assert(bridge.search_poll(123456789) == -3, "unknown id should return -3")
os.execute(('rm -rf "%s"'):format(tmp))
print("test_bridge: ok")
