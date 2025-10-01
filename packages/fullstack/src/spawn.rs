use std::future::Future;

/// Spawn a task in the background. If wasm is enabled, this will use the single threaded tokio runtime
pub(crate) fn spawn_platform<Fut>(
    f: impl FnOnce() -> Fut + Send + 'static,
) -> tokio::task::JoinHandle<Fut::Output>
where
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        use tokio_util::task::LocalPoolHandle;
        static TASK_POOL: std::sync::OnceLock<LocalPoolHandle> = std::sync::OnceLock::new();

        let pool = TASK_POOL.get_or_init(|| {
            LocalPoolHandle::new(
                std::thread::available_parallelism()
                    .map(usize::from)
                    .unwrap_or(1),
            )
        });

        pool.spawn_pinned(f)
    }

    #[cfg(target_arch = "wasm32")]
    {
        tokio::task::spawn_local(f())
    }
}
