use std::process::{Command, Stdio};

use mlua::{FromLua, UserData};

#[derive(Debug, Clone, Copy)]
enum StdioFromString {
    Inherit,
    Piped,
    Null,
}
impl<'lua> FromLua<'lua> for StdioFromString {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::String(v) = lua_value {
            let v = v.to_str().unwrap();
            return Ok(match v.to_lowercase().as_str() {
                "inherit" => Self::Inherit,
                "piped" => Self::Piped,
                "null" => Self::Null,
                _ => Self::Inherit,
            });
        }
        Ok(Self::Inherit)
    }
}
impl StdioFromString {
    pub fn to_stdio(self) -> Stdio {
        match self {
            StdioFromString::Inherit => Stdio::inherit(),
            StdioFromString::Piped => Stdio::piped(),
            StdioFromString::Null => Stdio::null(),
        }
    }
}

pub struct PluginCommander;
impl UserData for PluginCommander {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function(
            "exec",
            |_, args: (Vec<String>, StdioFromString, StdioFromString)| {
                let cmd = args.0;
                let stdout = args.1;
                let stderr = args.2;

                if cmd.is_empty() {
                    return Ok(());
                }
                let cmd_name = cmd.first().unwrap();
                let mut command = Command::new(cmd_name);
                let t = cmd
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i > 0)
                    .map(|v| v.1.clone())
                    .collect::<Vec<String>>();
                command.args(t);
                command.stdout(stdout.to_stdio()).stderr(stderr.to_stdio());
                command.output()?;
                Ok(())
            },
        );
    }

    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}
}
