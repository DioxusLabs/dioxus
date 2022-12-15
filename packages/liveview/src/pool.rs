use tokio_util::task::LocalPoolHandle;

#[derive(Clone)]
pub struct LiveView {
    pub(crate) pool: LocalPoolHandle,
}

impl Default for LiveView {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveView {
    pub fn new() -> Self {
        LiveView {
            pool: LocalPoolHandle::new(16),
        }
    }
}
