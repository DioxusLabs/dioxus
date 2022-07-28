use mlua::UserData;

pub struct PluginDownloader;
impl UserData for PluginDownloader {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(_methods: &mut M) {
        // methods.add_function("name", function)
    }   
}