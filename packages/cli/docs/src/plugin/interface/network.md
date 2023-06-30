# Network Functions

> you can use Network functions to download & read some data from internet

### download_file(url: string, path: string) -> boolean

This function can help you download some file from url, and it will return a *boolean* value to check the download status. (true: success | false: fail)

You need pass a target url and a local path (where you want to save this file)

```lua
-- this file will download to plugin temp directory
local status = plugin.network.download_file(
    "http://xxx.com/xxx.zip",
    plugin.dirs.temp_dir()
)
if status != true then
    log.error("Download Failed")
end
```

### clone_repo(url: string, path: string) -> boolean

This function can help you use `git clone` command (this system must have been installed git)

```lua
local status = plugin.network.clone_repo(
    "http://github.com/mrxiaozhuox/dioxus-starter",
    plugin.dirs.bin_dir()
)
if status != true then
    log.error("Clone Failed")
end
```