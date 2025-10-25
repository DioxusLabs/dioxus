mod callback;

use jni::{
    objects::{GlobalRef, JObject, JValue},
    JNIEnv, JavaVM,
};
use std::sync::OnceLock;

/// Cached reference to the Android activity.
static ACTIVITY: OnceLock<GlobalRef> = OnceLock::new();
static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// Request location permission at runtime
pub fn request_permission() -> bool {
    with_activity(|env, activity| {
        let permission = env
            .new_string("android.permission.ACCESS_FINE_LOCATION")
            .ok()?;
        let permissions_array = env
            .new_object_array(1, "java/lang/String", &JObject::null())
            .ok()?;
        env.set_object_array_element(&permissions_array, 0, permission)
            .ok()?;

        const REQUEST_CODE: i32 = 3;
        let activity_class = env.find_class("androidx/core/app/ActivityCompat").ok()?;

        env.call_static_method(
            activity_class,
            "requestPermissions",
            "(Landroid/app/Activity;[Ljava/lang/String;I)V",
            &[
                JValue::Object(activity),
                JValue::Object(&permissions_array),
                JValue::Int(REQUEST_CODE),
            ],
        )
        .ok()?;

        Some(true)
    })
    .unwrap_or(false)
}

/// Get the last known location
pub fn last_known() -> Option<(f64, f64)> {
    with_activity(|env, activity| {
        let service_name = env.new_string("location").ok()?;
        let location_manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_name)],
            )
            .ok()?
            .l()
            .ok()?;

        let provider = env.new_string("gps").ok()?;
        let mut location = env
            .call_method(
                &location_manager,
                "getLastKnownLocation",
                "(Ljava/lang/String;)Landroid/location/Location;",
                &[JValue::Object(&provider)],
            )
            .ok()?
            .l()
            .ok()?;

        if location.is_null() {
            let fused_provider = env.new_string("fused").ok()?;
            location = env
                .call_method(
                    &location_manager,
                    "getLastKnownLocation",
                    "(Ljava/lang/String;)Landroid/location/Location;",
                    &[JValue::Object(&fused_provider)],
                )
                .ok()?
                .l()
                .ok()?;
        }

        if location.is_null() {
            return None;
        }

        let latitude = env
            .call_method(&location, "getLatitude", "()D", &[])
            .ok()?
            .d()
            .ok()?;
        let longitude = env
            .call_method(&location, "getLongitude", "()D", &[])
            .ok()?
            .d()
            .ok()?;

        Some((latitude, longitude))
    })
}

/// Execute a JNI operation with a cached activity reference.
fn with_activity<F, R>(f: F) -> Option<R>
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
