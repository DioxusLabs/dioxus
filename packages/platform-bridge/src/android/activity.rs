use jni::{objects::JObject, JNIEnv, JavaVM};
use std::sync::OnceLock;

/// Cached reference to the Android activity.
static ACTIVITY: OnceLock<jni::objects::GlobalRef> = OnceLock::new();
static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// Execute a JNI operation with a cached activity reference.
///
/// This function handles the boilerplate of getting the JavaVM and Activity
/// references, caching them for subsequent calls. It's the foundation for
/// most Android mobile API operations.
///
/// # Arguments
///
/// * `f` - A closure that receives a mutable JNIEnv and the Activity JObject
///
/// # Returns
///
/// Returns `Some(R)` if the operation succeeds, `None` if there's an error
/// getting the VM or Activity references.
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

    let activity = if let Some(activity) = ACTIVITY.get() {
        activity
    } else {
        let raw_activity = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };
        let global = env.new_global_ref(&raw_activity).ok()?;
        match ACTIVITY.set(global) {
            Ok(()) => ACTIVITY.get().unwrap(),
            Err(global) => {
                drop(global);
                ACTIVITY.get()?
            }
        }
    };

    let activity_obj = activity.as_obj();
    f(&mut env, &activity_obj)
}
