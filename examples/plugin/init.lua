local api = require("interface")
local log = api.log;

Manager = {}

Manager.info = {
    name = "Dioxus-CLI Plugin Demo",
    repository = "http://github.com/DioxusLabs/cli",
    author = "YuKun Liu <mrxzx.info@gmail.com>",
}

Manager.onbuild = function ()
    print("")
end