# OS Functions

> you can use OS functions to get some system information

### current_platform() -> string ("windows" | "macos" | "linux")

This function can help you get system & platform type:

```lua
local platform = plugin.os.current_platform()
```