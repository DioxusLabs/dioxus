use mlua::UserData;

pub struct PluginOS;
impl UserData for PluginOS {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("current_platform", |_, ()| {
            if cfg!(target_os = "windows") {
                Ok("windows")
            } else if cfg!(target_os = "macos") {
                Ok("macos")
            } else if cfg!(target_os = "linux") {
                Ok("linux")
            } else {
                panic!("unsupported platformm");
            }
        });
    }
}
