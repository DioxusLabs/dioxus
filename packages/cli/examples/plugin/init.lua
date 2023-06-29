local Api = require("./interface")
local log = Api.log;

local manager = {
    name = "Dioxus-CLI Plugin Demo",
    repository = "http://github.com/DioxusLabs/cli",
    author = "YuKun Liu <mrxzx.info@gmail.com>",
}

manager.onLoad = function ()
    log.info("plugin loaded.")
end

manager.onStartBuild = function ()
    log.warn("system start to build")
end

return manager