# Command Functions

> you can use command functions to execute some code & script

Type Define:
```
Stdio: "Inhert" | "Piped" | "Null"
```

### `exec(commands: [string], stdout: Stdio, stderr: Stdio)`

you can use this function to run some command on the current system.

```lua
local cmd = plugin.command

manager.test = function ()
    cmd.exec({"git", "clone", "https://github.com/DioxusLabs/cli-plugin-library"})
end
```
> Warning: This function don't have exception catch.