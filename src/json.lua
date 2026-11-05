-- Small JSON encoder and decoder. Keys are sorted, so equal tables encode to equal strings.
local M = {}

local ESCAPES = { ['"'] = '\\"', ["\\"] = "\\\\", ["\n"] = "\\n", ["\r"] = "\\r", ["\t"] = "\\t" }

local function encode_string(s)
  return '"' .. s:gsub('[%c"\\]', function(c)
    return ESCAPES[c] or string.format("\\u%04x", c:byte())
  end) .. '"'
end

local function encode_number(n)
  if n ~= n or n == math.huge or n == -math.huge then return '"naneinf"' end
  if n == math.floor(n) and math.abs(n) < 1e15 then return string.format("%d", n) end
  return string.format("%.17g", n)
end

local function is_array(t)
  local count = 0
  for k in next, t do
    if type(k) ~= "number" then return false end
    count = count + 1
  end
  return count == #t
end

local encode_value

local function encode_table(t, out)
  if is_array(t) then
    out[#out + 1] = "["
    for i = 1, #t do
      if i > 1 then out[#out + 1] = "," end
      encode_value(t[i], out)
    end
    out[#out + 1] = "]"
    return
  end
  local keys = {}
  for k in next, t do keys[#keys + 1] = tostring(k) end
  table.sort(keys)
  local lookup = {}
  for k, v in next, t do lookup[tostring(k)] = v end
  out[#out + 1] = "{"
  for i, k in ipairs(keys) do
    if i > 1 then out[#out + 1] = "," end
    out[#out + 1] = encode_string(k)
    out[#out + 1] = ":"
    encode_value(lookup[k], out)
  end
  out[#out + 1] = "}"
end

encode_value = function(v, out)
  local kind = type(v)
  if kind == "string" then
    out[#out + 1] = encode_string(v)
  elseif kind == "number" then
    out[#out + 1] = encode_number(v)
  elseif kind == "boolean" then
    out[#out + 1] = v and "true" or "false"
  elseif kind == "table" then
    encode_table(v, out)
  else
    out[#out + 1] = "null"
  end
end

function M.encode(v)
  local out = {}
  encode_value(v, out)
  return table.concat(out)
end

local UNESCAPES = { b = "\b", f = "\f", n = "\n", r = "\r", t = "\t", ['"'] = '"', ["\\"] = "\\", ["/"] = "/" }

local function utf8_char(code)
  if code < 0x80 then return string.char(code) end
  if code < 0x800 then
    return string.char(0xC0 + math.floor(code / 0x40), 0x80 + code % 0x40)
  end
  return string.char(0xE0 + math.floor(code / 0x1000), 0x80 + math.floor(code / 0x40) % 0x40, 0x80 + code % 0x40)
end

function M.decode(text)
  local pos = 1

  local function fail(message)
    error("json: " .. message .. " at " .. pos, 0)
  end

  local function skip()
    pos = text:find("[^ \t\r\n]", pos) or #text + 1
  end

  local parse_value

  local function parse_string()
    pos = pos + 1
    local parts = {}
    while true do
      local stop = text:find('["\\]', pos)
      if not stop then fail("unterminated string") end
      parts[#parts + 1] = text:sub(pos, stop - 1)
      pos = stop + 1
      if text:sub(stop, stop) == '"' then return table.concat(parts) end
      local esc = text:sub(pos, pos)
      if esc == "u" then
        local code = tonumber(text:sub(pos + 1, pos + 4), 16)
        if not code then fail("bad unicode escape") end
        parts[#parts + 1] = utf8_char(code)
        pos = pos + 5
      else
        local char = UNESCAPES[esc]
        if not char then fail("bad escape") end
        parts[#parts + 1] = char
        pos = pos + 1
      end
    end
  end

  local function parse_array()
    pos = pos + 1
    local out, n = {}, 0
    skip()
    if text:sub(pos, pos) == "]" then pos = pos + 1; return out end
    while true do
      n = n + 1
      out[n] = parse_value()
      skip()
      local c = text:sub(pos, pos)
      pos = pos + 1
      if c == "]" then return out end
      if c ~= "," then fail("expected , or ]") end
    end
  end

  local function parse_object()
    pos = pos + 1
    local out = {}
    skip()
    if text:sub(pos, pos) == "}" then pos = pos + 1; return out end
    while true do
      skip()
      if text:sub(pos, pos) ~= '"' then fail("expected key") end
      local key = parse_string()
      skip()
      if text:sub(pos, pos) ~= ":" then fail("expected :") end
      pos = pos + 1
      out[key] = parse_value()
      skip()
      local c = text:sub(pos, pos)
      pos = pos + 1
      if c == "}" then return out end
      if c ~= "," then fail("expected , or }") end
    end
  end

  parse_value = function()
    skip()
    local c = text:sub(pos, pos)
    if c == "{" then return parse_object() end
    if c == "[" then return parse_array() end
    if c == '"' then return parse_string() end
    if text:sub(pos, pos + 3) == "true" then pos = pos + 4; return true end
    if text:sub(pos, pos + 4) == "false" then pos = pos + 5; return false end
    if text:sub(pos, pos + 3) == "null" then pos = pos + 4; return nil end
    local number = text:match("^-?%d+%.?%d*[eE]?[+-]?%d*", pos)
    if not number or not tonumber(number) then fail("unexpected value") end
    pos = pos + #number
    return tonumber(number)
  end

  local value = parse_value()
  skip()
  if pos <= #text then fail("trailing data") end
  return value
end

return M
