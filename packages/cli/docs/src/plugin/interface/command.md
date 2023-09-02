# Command Functions

You can use command functions to execute code and scripts.

Type definition:
```
Stdio: "Inherit" | "Piped" | "Null"
```

### `exec(commands: [string], stdout: Stdio, stderr: Stdio)`

You can use this function to run some commands on the current system.

```lua
local cmd = plugin.command

manager.test = function ()
    cmd.exec({"git", "clone", "https://github.com/DioxusLabs/cli-plugin-library"})
end
```
> Warning: This function doesn't catch exceptions.