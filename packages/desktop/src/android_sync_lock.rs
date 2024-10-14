/// This is a hack to get around the fact that wry is currently not thread safe on android
///
/// We want to acquire this mutex before doing anything with the virtualdom directly
pub fn android_runtime_lock() -> std::sync::MutexGuard<'static, ()> {
    use once_cell::sync::OnceCell;
    use std::sync::Mutex;

    static RUNTIME_LOCK: OnceCell<Mutex<()>> = OnceCell::new();

    RUNTIME_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}
