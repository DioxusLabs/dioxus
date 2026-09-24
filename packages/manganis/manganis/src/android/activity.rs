use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject},
};
use std::sync::{OnceLock, RwLock};

/// The live Android activity, replaced whenever the system recreates it.
static CURRENT_ACTIVITY: RwLock<Option<GlobalRef>> = RwLock::new(None);
static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// Make `activity` the one [`with_activity`] hands out, called by the renderer for every new activity.
pub fn set_current_activity(activity: GlobalRef) {
    *CURRENT_ACTIVITY
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(activity);
}

/// Execute a JNI operation with the current activity.
///
/// Without an activity registered through [`set_current_activity`], the closure receives the
/// `ndk_context` context.
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

    // The clone keeps the reference alive while `f` runs, even if the activity is replaced meanwhile.
    let activity = CURRENT_ACTIVITY
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    match activity {
        Some(activity) => f(&mut env, activity.as_obj()),
        None => {
            // SAFETY: `ndk_context` holds this global reference for as long as it is initialised.
            let context = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };
            f(&mut env, &context)
        }
    }
}
