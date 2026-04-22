/// This is a hack to get around the fact that wry is currently not thread safe on android
///
/// We want to acquire this mutex before doing anything with the virtualdom directly
#[cfg(target_os = "android")]
pub fn android_runtime_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};

    static RUNTIME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    RUNTIME_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}
