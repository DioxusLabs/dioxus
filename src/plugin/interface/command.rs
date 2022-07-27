use std::process::{Command, Stdio};

use mlua::UserData;

pub struct PluginCommander;
impl UserData for PluginCommander {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("exec", |_, cmd: Vec<String>| {
            if cmd.len() == 0 {
                return Ok(());
            }
            let cmd_name = cmd.get(0).unwrap();
            let mut command = Command::new(cmd_name);
            let t = cmd
                .iter()
                .enumerate()
                .filter(|(i, _)| *i > 0)
                .map(|v| v.1.clone())
                .collect::<Vec<String>>();
            command.args(t);
            command.stdout(Stdio::inherit());
            command.output()?;
            Ok(())
        });
        methods.add_function("execQuiet", |_, cmd: Vec<String>| {
            if cmd.len() == 0 {
                return Ok(());
            }
            let cmd_name = cmd.get(0).unwrap();
            let mut command = Command::new(cmd_name);
            let t = cmd
                .iter()
                .enumerate()
                .filter(|(i, _)| *i > 0)
                .map(|v| v.1.clone())
                .collect::<Vec<String>>();
            command.args(t);
            command.stdout(Stdio::null());
            command.output()?;
            Ok(())
        });
        methods.add_function("execSimple", |_, cmd: String| {
            let _ = Command::new(cmd).stdout(Stdio::inherit()).output()?;
            Ok(())
        });
    }

    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {
        
    }
}
