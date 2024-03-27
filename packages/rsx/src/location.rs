/// Information about the location of the call to a component
///
/// This will be filled in when the dynamiccontext is built, filling in the file:line:column:id format
///
#[derive(PartialEq, Eq, Clone, Debug, Hash, Default)]
pub struct CallerLocation {
    inner: Option<String>,
}
