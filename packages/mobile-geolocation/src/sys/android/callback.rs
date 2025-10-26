use std::sync::OnceLock;

use jni::{
    objects::{GlobalRef, JClass, JObject, JValue},
    JNIEnv,
};

use crate::error::Result;
use dioxus_mobile_core::android::CallbackSystem;

/// The compiled DEX bytecode included at compile time
const CALLBACK_BYTECODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/classes.dex"));

/// Must match the method name in LocationCallback.java
const RUST_CALLBACK_NAME: &str = "rustCallback";

/// Must match the signature of rust_callback and LocationCallback.java
const RUST_CALLBACK_SIGNATURE: &str = "(JJLandroid/location/Location;)V";

/// Global reference to the callback class (loaded once)
static CALLBACK_CLASS: OnceLock<GlobalRef> = OnceLock::new();

fn load_class_from_dex<'env>(
    env: &mut JNIEnv<'env>,
    bytecode: &'static [u8],
    class_name: &str,
) -> Result<JClass<'env>> {
    const IN_MEMORY_LOADER: &str = "dalvik/system/InMemoryDexClassLoader";

    let byte_buffer =
        unsafe { env.new_direct_byte_buffer(bytecode.as_ptr() as *mut u8, bytecode.len()) }?;

    let dex_class_loader = env.new_object(
        IN_MEMORY_LOADER,
        "(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V",
        &[
            JValue::Object(&byte_buffer),
            JValue::Object(&JObject::null()),
        ],
    )?;

    let class_name = env.new_string(class_name)?;
    let class = env
        .call_method(
            &dex_class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name)],
        )?
        .l()?;

    Ok(class.into())
}

/// Get or load the callback class
pub(super) fn get_callback_class(env: &mut JNIEnv<'_>) -> Result<&'static GlobalRef> {
    if let Some(class) = CALLBACK_CLASS.get() {
        return Ok(class);
    }

    let callback_system = CallbackSystem::new(
        CALLBACK_BYTECODE,
        "dioxus.mobile.geolocation.LocationCallback",
        RUST_CALLBACK_NAME,
        RUST_CALLBACK_SIGNATURE,
    );

    let global = callback_system.load_and_register(env)?;
    Ok(CALLBACK_CLASS.get_or_init(|| global))
}

pub(super) fn load_permissions_helper_class<'env>(env: &mut JNIEnv<'env>) -> Result<JClass<'env>> {
    load_class_from_dex(
        env,
        CALLBACK_BYTECODE,
        "dioxus.mobile.geolocation.PermissionsHelper",
    )
}
