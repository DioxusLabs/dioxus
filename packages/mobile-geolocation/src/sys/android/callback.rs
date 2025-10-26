use std::sync::OnceLock;

use jni::{
    objects::{GlobalRef, JClass, JObject},
    JNIEnv,
};

use crate::error::Result;
use dioxus_mobile_core::android::with_activity;

/// Must match the method name in LocationCallback.java
const RUST_CALLBACK_NAME: &str = "rustCallback";

/// Must match the signature of rust_callback and LocationCallback.java
const RUST_CALLBACK_SIGNATURE: &str = "(JJLandroid/location/Location;)V";

/// Global reference to the callback class (loaded once)
static CALLBACK_CLASS: OnceLock<GlobalRef> = OnceLock::new();

/// Load a class using the app's default class loader
/// This works because Gradle compiles Java sources and includes them in the APK
fn load_class_from_classloader<'env>(
    env: &mut JNIEnv<'env>,
    class_name: &str,
) -> Result<JClass<'env>> {
    // Get the current thread's context class loader
    // This will find classes that are part of the APK
    let class_name_jstring = env.new_string(class_name)?;

    // Try to load the class using Class.forName()
    let class = env
        .call_static_method(
            "java/lang/Class",
            "forName",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[(&class_name_jstring).into()],
        )?
        .l()?;

    Ok(class.into())
}

/// Get or load the callback class
pub(super) fn get_callback_class(env: &mut JNIEnv<'_>) -> Result<&'static GlobalRef> {
    if let Some(class) = CALLBACK_CLASS.get() {
        return Ok(class);
    }

    // Load the callback class from the APK
    let callback_class =
        load_class_from_classloader(env, "dioxus.mobile.geolocation.LocationCallback")?;

    // Register the native callback method
    use jni::NativeMethod;
    env.register_native_methods(
        &callback_class,
        &[NativeMethod {
            name: RUST_CALLBACK_NAME.into(),
            sig: RUST_CALLBACK_SIGNATURE.into(),
            fn_ptr: rust_callback as *mut _,
        }],
    )?;

    let global = env.new_global_ref(callback_class)?;
    Ok(CALLBACK_CLASS.get_or_init(|| global))
}

pub(super) fn load_permissions_helper_class<'env>(env: &mut JNIEnv<'env>) -> Result<JClass<'env>> {
    load_class_from_classloader(env, "dioxus.mobile.geolocation.PermissionsHelper")
}

/// Native callback function called from Java
///
/// SAFETY: This function is called from Java and must maintain proper memory safety.
#[no_mangle]
unsafe extern "C" fn rust_callback<'a>(
    mut _env: JNIEnv<'a>,
    _class: JObject<'a>,
    _handler_ptr_high: jni::sys::jlong,
    _handler_ptr_low: jni::sys::jlong,
    _location: JObject<'a>,
) {
    // This callback is registered but not currently used
    // Future implementations can use this for async location updates
}
