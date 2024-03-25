use mlua::UserData;

pub struct PluginLogger;
impl UserData for PluginLogger {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("trace", |_, info: String| {
            tracing::trace!("{}", info);
            Ok(())
        });
        methods.add_function("info", |_, info: String| {
            tracing::info!("{}", info);
            Ok(())
        });
        methods.add_function("debug", |_, info: String| {
            tracing::debug!("{}", info);
            Ok(())
        });
        methods.add_function("warn", |_, info: String| {
            tracing::warn!("{}", info);
            Ok(())
        });
        methods.add_function("error", |_, info: String| {
            tracing::error!("{}", info);
            Ok(())
        });
    }
}
