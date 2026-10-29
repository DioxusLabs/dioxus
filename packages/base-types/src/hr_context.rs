pub trait HotReloadingContext {
    fn map_attribute(
        element_name_rust: &str,
        attribute_name_rust: &str,
    ) -> Option<(&'static str, Option<&'static str>)>;
    fn map_element(element_name_rust: &str) -> Option<(&'static str, Option<&'static str>)>;
}

pub struct Empty;

impl HotReloadingContext for Empty {
    fn map_attribute(_: &str, _: &str) -> Option<(&'static str, Option<&'static str>)> {
        None
    }

    fn map_element(_: &str) -> Option<(&'static str, Option<&'static str>)> {
        None
    }
}
