-- Usage: luajit tests/lua/test_ffi.lua <path to built library>
local ffi = require("ffi")
ffi.cdef("uint32_t mqol_abi_version(void);")
local lib = ffi.load(assert(arg[1], "library path required"))
assert(lib.mqol_abi_version() == 1, "abi version should be 1")
print("test_ffi: ok")
