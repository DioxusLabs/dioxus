# Dirs Functions

Dirs functions are for getting various directory paths. Not to be confused with `plugin.path`.

### `plugin_dir() -> string`

Get the plugin's root directory path.

```lua
local path = plugin.dirs.plugin_dir()
-- example: ~/Development/DioxusCli/plugin/test-plugin/
```

### `bin_dir() -> string`

Get the plugin's binary directory path. Put binary files like `tailwind-cli` or `sass-cli` in this directory.

```lua
local path = plugin.dirs.bin_dir()
-- example: ~/Development/DioxusCli/plugin/test-plugin/bin/
```

### `temp_dir() -> string`

Get the plugin's temporary directory path. Put any temporary files here.

```lua
local path = plugin.dirs.bin_dir()
-- example: ~/Development/DioxusCli/plugin/test-plugin/temp/
```