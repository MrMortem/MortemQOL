-- Loads the native engine and wraps its C ABI. Never raises: failures report through return values.
local ffi = require("ffi")

local M = {}
local ABI_VERSION = 1
local lib = nil
local cdef_done = false
local buf_cap = 64 * 1024
local buf = nil
local out_len = nil

function M._loader(path)
  return ffi.load(path)
end

local function ensure_cdef()
  if cdef_done then return end
  ffi.cdef([[
    uint32_t mqol_abi_version(void);
    uint64_t mqol_search_start(const uint8_t *json, size_t len);
    int32_t mqol_search_poll(uint64_t id, uint8_t *out, size_t cap, size_t *out_len);
    int32_t mqol_score(const uint8_t *json, size_t len, uint8_t *out, size_t cap, size_t *out_len);
  ]])
  cdef_done = true
end

local function library_path(mod_path)
  if mod_path:sub(-1) ~= "/" then mod_path = mod_path .. "/" end
  if ffi.os == "OSX" then return mod_path .. "native/macos/libmortemqol_engine.dylib" end
  if ffi.os == "Windows" then return mod_path .. "native/windows/mortemqol_engine.dll" end
  return nil
end

-- Returns true, or false and a message.
function M.load(mod_path)
  lib = nil
  local path = library_path(mod_path)
  if not path then return false, "no native engine for " .. ffi.os end
  local ok, result = pcall(function()
    ensure_cdef()
    local loaded = M._loader(path)
    local version = tonumber(loaded.mqol_abi_version())
    if version ~= ABI_VERSION then
      error("engine ABI " .. tostring(version) .. ", expected " .. ABI_VERSION, 0)
    end
    return loaded
  end)
  if not ok then
    local message = tostring(result)
    if ffi.os == "OSX" and not message:find("ABI") then
      message = message .. " (if you extracted the zip by hand, macOS quarantine may be blocking it; import the zip into r2modmac instead)"
    end
    return false, message
  end
  lib = result
  buf = ffi.new("uint8_t[?]", buf_cap)
  out_len = ffi.new("size_t[1]")
  return true
end

function M.available()
  return lib ~= nil
end

-- Calls `call(out, cap, out_len)`, growing the buffer once if the engine asks for more room.
local function with_buffer(call)
  local status = call(buf, buf_cap, out_len)
  if status == -2 then
    buf_cap = tonumber(out_len[0]) + 1024
    buf = ffi.new("uint8_t[?]", buf_cap)
    status = call(buf, buf_cap, out_len)
  end
  local size = tonumber(out_len[0])
  if status == -2 or size == 0 then return status, nil end
  return status, ffi.string(buf, size)
end

-- Returns a search id, or nil when the engine is unavailable or refused the request.
function M.search_start(json)
  if not lib then return nil end
  local id = lib.mqol_search_start(json, #json)
  if id == 0ULL then return nil end
  return id
end

-- Returns status, json_or_nil. Status values are documented in engine/src/ffi.rs.
function M.search_poll(id)
  if not lib then return -1, "engine unavailable" end
  out_len[0] = 0
  return with_buffer(function(out, cap, len)
    return lib.mqol_search_poll(id, out, cap, len)
  end)
end

-- Returns status, json_or_nil.
function M.score(json)
  if not lib then return -1, "engine unavailable" end
  out_len[0] = 0
  return with_buffer(function(out, cap, len)
    return lib.mqol_score(json, #json, out, cap, len)
  end)
end

return M
