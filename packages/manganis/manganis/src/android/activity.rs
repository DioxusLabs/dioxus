use jni::{JNIEnv, JavaVM, objects::GlobalRef, objects::JObject};
use std::sync::{Mutex, OnceLock};

/// The current Android activity, as last passed to [`set_activity`].
static ACTIVITY: Mutex<Option<GlobalRef>> = Mutex::new(None);
static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// Set the activity that [`with_activity`] passes to its closure.
///
/// Call this every time an activity is created, since Android re-creates it on configuration
/// changes such as rotation. `dioxus-desktop` does this for you.
pub fn set_activity(activity: GlobalRef) {
    *ACTIVITY.lock().unwrap_or_else(|e| e.into_inner()) = Some(activity);
}

/// Execute a JNI operation with the current activity.
///
/// This function handles the boilerplate of getting the JavaVM and Activity
/// references. It's the foundation for most Android mobile API operations.
///
/// # Arguments
///
/// * `f` - A closure that receives a mutable JNIEnv and the Activity JObject
///
/// # Returns
///
/// Returns `Some(R)` if the operation succeeds, `None` if there's an error
/// getting the VM, or if no activity has been set with [`set_activity`].
///
/// # Example
///
/// ```rust,no_run
/// use dioxus_platform_bridge::android::with_activity;
///
/// let result = with_activity(|env, activity| {
///     // Your JNI operations here
///     Some(42)
/// });
/// ```
pub fn with_activity<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> Option<R>,
{
    let ctx = ndk_context::android_context();
    let vm = if let Some(vm) = JAVA_VM.get() {
        vm
    } else {
        let raw_vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
        let _ = JAVA_VM.set(raw_vm);
        JAVA_VM.get()?
    };
    let mut env = vm.attach_current_thread().ok()?;

    // Clone the reference out so that the lock is not held while `f` runs.
    let activity = ACTIVITY.lock().unwrap_or_else(|e| e.into_inner()).clone()?;

    let activity_obj = activity.as_obj();
    f(&mut env, &activity_obj)
}
