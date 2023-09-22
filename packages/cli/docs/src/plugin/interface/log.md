# Log Functions

You can use log functions to print various logging information.

### `trace(info: string)`

Print trace log info.

```lua
local log = plugin.log
log.trace("trace information")
```

### `debug(info: string)`

Print debug log info.

```lua
local log = plugin.log
log.debug("debug information")
```

### `info(info: string)`

Print info log info.

```lua
local log = plugin.log
log.info("info information")
```

### `warn(info: string)`

Print warning log info.

```lua
local log = plugin.log
log.warn("warn information")
```

### `error(info: string)`

Print error log info.

```lua
local log = plugin.log
log.error("error information")
```