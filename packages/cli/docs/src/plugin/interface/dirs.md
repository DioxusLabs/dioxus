# Dirs Functions

> you can use Dirs functions to get some directory path


### plugin_dir() -> string

You can get current plugin **root** directory path

```lua
local path = plugin.dirs.plugin_dir()
-- example: ~/Development/DioxusCli/plugin/test-plugin/
```

### bin_dir() -> string

You can get plugin **bin** direcotry path

Sometime you need install some binary file like `tailwind-cli` & `sass-cli` to help your plugin work, then you should put binary file in this directory.

```lua
local path = plugin.dirs.bin_dir()
-- example: ~/Development/DioxusCli/plugin/test-plugin/bin/
```

### temp_dir() -> string

You can get plugin **temp** direcotry path

Just put some temporary file in this directory.

```lua
local path = plugin.dirs.bin_dir()
-- example: ~/Development/DioxusCli/plugin/test-plugin/temp/
```