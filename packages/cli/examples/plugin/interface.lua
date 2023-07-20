local interface = {}

if plugin_logger ~= nil then
    interface.log = plugin_logger
else
    interface.log = {
        trace = function (info)
            print("trace: " .. info)
        end,
        debug = function (info)
            print("debug: " .. info)
        end,
        info = function (info)
            print("info: " .. info)
        end,
        warn = function (info)
            print("warn: " .. info)
        end,
        error = function (info)
            print("error: " .. info)
        end,
    }
end

return interface