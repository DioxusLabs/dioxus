# Network Functions

You can use Network functions to download & read some data from the internet.

### `download_file(url: string, path: string) -> boolean`

Downloads a file from the specified URL,
and returns a `boolean` that represents the download status (true: success, false: failure).

You need to pass a target URL and a local path (where you want to save this file).

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

### `clone_repo(url: string, path: string) -> boolean`

Clone a repository from the given URL into the given path.
Returns a `boolean` that represents the clone status (true: success, false: failure).
The system executing this function must have git installed.

```lua
local status = plugin.network.clone_repo(
    "http://github.com/mrxiaozhuox/dioxus-starter",
    plugin.dirs.bin_dir()
)
if status != true then
    log.error("Clone Failed")
end
```