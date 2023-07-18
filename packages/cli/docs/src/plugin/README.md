# CLI Plugin Development

> For Cli 0.2.0 we will add `plugin-develop` support.

Before the 0.2.0 we use `dioxus tool` to use & install some plugin, but we think that is not good for extend cli program, some people want tailwind support, some people want sass support, we can't add all this thing in to the cli source code and we don't have time to maintain a lot of tools that user request, so maybe user make plugin by themself is a good choice.

### Why Lua ?

We choose `Lua: 5.4` to be the plugin develop language, because cli plugin is not complex, just like a workflow, and user & developer can write some easy code for their plugin. We have **vendored** lua in cli program, and user don't need install lua runtime in their computer, and the lua parser & runtime doesn't take up much disk memory.

### Event Management

The plugin library have pre-define some important event you can control:

- `build.on_start`
- `build.on_finished`
- `serve.on_start`
- `serve.on_rebuild`
- `serve.on_shutdown`

### Plugin Template

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