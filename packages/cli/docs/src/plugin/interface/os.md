# OS Functions

OS functions are for getting system information.

### `current_platform() -> string ("windows" | "macos" | "linux")`

Get the current OS platform.

```lua
local platform = plugin.os.current_platform()
```