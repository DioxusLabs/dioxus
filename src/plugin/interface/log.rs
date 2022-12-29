use log;
use mlua::UserData;

pub struct PluginLogger;
impl UserData for PluginLogger {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("trace", |_, info: String| {
            log::trace!("{}", info);
            Ok(())
        });
        methods.add_function("info", |_, info: String| {
            log::info!("{}", info);
            Ok(())
        });
        methods.add_function("debug", |_, info: String| {
            log::debug!("{}", info);
            Ok(())
        });
        methods.add_function("warn", |_, info: String| {
            log::warn!("{}", info);
            Ok(())
        });
        methods.add_function("error", |_, info: String| {
            log::error!("{}", info);
            Ok(())
        });
    }
}
