# Path Functions

You can use path functions to perform operations on valid path strings.

### `join(path: string, extra: string) -> string`

<!-- TODO: Add specifics.
From the example given, it seems like it just creates a subdirectory path.
What would it do when "extending" file paths? -->
Extend a path; you can extend both directory and file paths.

```lua
local current_path = "~/hello/dioxus"
local new_path = plugin.path.join(current_path, "world")
-- new_path = "~/hello/dioxus/world"
```

### `parent(path: string) -> string`

Return the parent path of the specified path. The parent path is always a directory.

```lua
local current_path = "~/hello/dioxus"
local new_path = plugin.path.parent(current_path)
-- new_path = "~/hello/"
```

### `exists(path: string) -> boolean`

Check if the specified path exists, as either a file or a directory.

### `is_file(path: string) -> boolean`

Check if the specified path is a file.

### `is_dir(path: string) -> boolean`

Check if the specified path is a directory.