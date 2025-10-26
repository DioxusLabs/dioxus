use std::sync::OnceLock;

use jni::{
    objects::{GlobalRef, JClass, JObject},
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
