# CLI Plugin development

**IMPORTANT: Ignore this documentation. Plugins are yet to be released and chances are it won't work for you. This is just what plugins *could* look like.**

In the past we used `dx tool` to use and install tools, but it was a flawed system.
Tools were hard-coded by us, but people want more tools than we could code, so this plugin system was made to let
anyone develop plugins and use them in Dioxus projects.

Plugin resources:
* [Source code](https://github.com/DioxusLabs/dioxus/tree/master/packages/cli/src/plugin)
* [Unofficial Dioxus plugin community](https://github.com/DioxusPluginCommunity). Contains certain plugins you can use right now.

### Why Lua?

We chose Lua `5.4` to be the plugin developing language,
because it's extremely lightweight, embeddable and easy to learn.
We installed Lua into the CLI, so you don't need to do it yourself.

Lua resources:
* [Official website](https://www.lua.org/). You can basically find everything here.
* [Awesome Lua](https://github.com/LewisJEllis/awesome-lua). Additional resources (such as Lua plugins for your favorite IDE), and other *awesome* tools!


## Creating a plugin

A plugin is just an `init.lua` file.
You can include other files using `dofile(path)`.
You need to have a plugin and a manager instance, which you can get using `require`:
```lua
local plugin = require("plugin")
local manager = require("manager")
```

You need to set some `manager` fields and then initialize the plugin:
```lua
manager.name = "My first plugin"
manager.repository = "https://github.com/john-doe/my-first-plugin" -- The repository URL.
manager.author = "John Doe <john.doe@example.com>"
manager.version = "0.1.0"
plugin.init(manager)
```

You also need to return the `manager`, which basically represents your plugin:
```lua
-- Your code here.
-- End of file.

manager.serve.interval = 1000
return manager
```

And you're ready to go. Now, go and have a look at the stuff below and the API documentation.

### Plugin info

You will encounter this type in the events below. The keys are as follows:
* `name: string` - The name of the plugin.
* `repository: string` - The plugin repository URL.
* `author: string` - The author of the plugin.
* `version: string` - The plugin version.

### Event management

The plugin library has certain events that you can subscribe to.

* `manager.on_init` - Triggers the first time the plugin is loaded.
* `manager.build.on_start(info)` - Triggers before the build process. E.g., before `dx build`.
* `manager.build.on_finish(info)` - Triggers after the build process. E.g., after `dx build`.
* `manager.serve.on_start(info)` - Triggers before the serving process. E.g., before `dx serve`.
* `manager.serve.on_rebuild_start(info)` - Triggers before the server rebuilds the web with hot reload.
* `manager.serve.on_rebuild_end(info)` - Triggers after the server rebuilds the web with hot reload.
* `manager.serve.on_shutdown` - Triggers when the server is shutdown. E.g., when the `dx serve` process is terminated.

To subscribe to an event, you simply need to assign it to a function:

```lua
manager.build.on_start = function (info)
    log.info("[plugin] Build starting: " .. info.name)
end
```

### Plugin template

```lua
package.path = library_dir .. "/?.lua"

local plugin = require("plugin")
local manager = require("manager")

-- deconstruct api functions
local log = plugin.log

-- plugin information
manager.name = "Hello Dixous Plugin"
manager.repository = "https://github.com/mrxiaozhuox/hello-dioxus-plugin"
manager.author = "YuKun Liu <mrxzx.info@gmail.com>"
manager.version = "0.0.1"

-- init manager info to plugin api
plugin.init(manager)

manager.on_init = function ()
    -- when the first time plugin been load, this function will be execute.
    -- system will create a `dcp.json` file to verify init state.
    log.info("[plugin] Start to init plugin: " .. manager.name)
end

---@param info BuildInfo
manager.build.on_start = function (info)
    -- before the build work start, system will execute this function.
    log.info("[plugin] Build starting: " .. info.name)
end

---@param info BuildInfo
manager.build.on_finish = function (info)
    -- when the build work is done, system will execute this function.
    log.info("[plugin] Build finished: " .. info.name)
end

---@param info ServeStartInfo
manager.serve.on_start = function (info)
    -- this function will after clean & print to run, so you can print some thing.
    log.info("[plugin] Serve start: " .. info.name)
end

---@param info ServeRebuildInfo
manager.serve.on_rebuild = function (info)
    -- this function will after clean & print to run, so you can print some thing.
    local files = plugin.tool.dump(info.changed_files)
    log.info("[plugin] Serve rebuild: '" .. files .. "'")
end

manager.serve.on_shutdown = function ()
    log.info("[plugin] Serve shutdown")
end

manager.serve.interval = 1000

return manager
```
