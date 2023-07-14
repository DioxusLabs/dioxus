# Path Functions

> you can use path functions to operate valid path string

### join(path: string, extra: string) -> string

This function can help you extend a path, you can extend any path, dirname or filename.

```lua
local current_path = "~/hello/dioxus"
local new_path = plugin.path.join(current_path, "world")
-- new_path = "~/hello/dioxus/world"
```

### parent(path: string) -> string

This function will return `path` parent-path string, back to the parent.

```lua
local current_path = "~/hello/dioxus"
local new_path = plugin.path.parent(current_path)
-- new_path = "~/hello/"
```

### exists(path: string) -> boolean

This function can check some path (dir & file) is exists.

### is_file(path: string) -> boolean

This function can check some path is a exist file.

### is_dir(path: string) -> boolean

This function can check some path is a exist dir.