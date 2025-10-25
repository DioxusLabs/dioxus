use std::sync::OnceLock;

use jni::{
    objects::{GlobalRef, JClass, JObject, JValue},
    sys::jlong,
    JNIEnv, NativeMethod,
};

use crate::error::Result;

/// The compiled DEX bytecode included at compile time
const CALLBACK_BYTECODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/classes.dex"));

/// Must match the method name in LocationCallback.java
const RUST_CALLBACK_NAME: &str = "rustCallback";

/// Must match the signature of rust_callback and LocationCallback.java
const RUST_CALLBACK_SIGNATURE: &str = "(JJLandroid/location/Location;)V";

/// Native callback function called from Java
///
/// SAFETY: This function is called from Java and must maintain proper memory safety.
/// The handler pointer is valid as long as the Manager exists (see Drop implementation).
#[no_mangle]
unsafe extern "C" fn rust_callback<'a>(
    mut env: JNIEnv<'a>,
    _class: JObject<'a>,
    handler_ptr_high: jlong,
    handler_ptr_low: jlong,
    location: JObject<'a>,
) {
    // Reconstruct the pointer from two i64 values (for 64-bit pointers)
    #[cfg(not(target_pointer_width = "64"))]
    compile_error!("Only 64-bit Android targets are supported");

    let handler_ptr_raw: usize =
        ((handler_ptr_high as u64) << 32 | handler_ptr_low as u64) as usize;

    // Convert to our callback function pointer
    let callback: fn(&mut JNIEnv, JObject) = unsafe { std::mem::transmute(handler_ptr_raw) };

    // Create a global reference to the location object
    if let Ok(global_location) = env.new_global_ref(&location) {
        callback(&mut env, unsafe {
            JObject::from_raw(global_location.as_obj().as_raw())
        });
    }
}

/// Global reference to the callback class (loaded once)
static CALLBACK_CLASS: OnceLock<GlobalRef> = OnceLock::new();

/// Get or load the callback class
pub(super) fn get_callback_class(env: &mut JNIEnv<'_>) -> Result<&'static GlobalRef> {
    if let Some(class) = CALLBACK_CLASS.get() {
        return Ok(class);
    }

    let callback_class = load_callback_class(env)?;
    register_rust_callback(env, &callback_class)?;
    let global = env.new_global_ref(callback_class)?;

    Ok(CALLBACK_CLASS.get_or_init(|| global))
}

/// Register the native rust_callback method with the Java class
fn register_rust_callback<'a>(env: &mut JNIEnv<'a>, callback_class: &JClass<'a>) -> Result<()> {
    env.register_native_methods(
        callback_class,
        &[NativeMethod {
            name: RUST_CALLBACK_NAME.into(),
            sig: RUST_CALLBACK_SIGNATURE.into(),
            fn_ptr: rust_callback as *mut _,
        }],
    )?;
    Ok(())
}

/// Load the callback class from the compiled DEX bytecode
fn load_callback_class<'a>(env: &mut JNIEnv<'a>) -> Result<JClass<'a>> {
    const IN_MEMORY_LOADER: &str = "dalvik/system/InMemoryDexClassLoader";

    // Create a ByteBuffer from our DEX bytecode
    let byte_buffer = unsafe {
        env.new_direct_byte_buffer(
            CALLBACK_BYTECODE.as_ptr() as *mut u8,
            CALLBACK_BYTECODE.len(),
        )
    }?;

    // Create an InMemoryDexClassLoader with our DEX bytecode
    let dex_class_loader = env.new_object(
        IN_MEMORY_LOADER,
        "(Ljava/nio/ByteBuffer;Ljava/lang/ClassLoader;)V",
        &[
            JValue::Object(&byte_buffer),
            JValue::Object(&JObject::null()),
        ],
    )?;

    // Load our LocationCallback class
    let class_name = env.new_string("dioxus.mobile.geolocation.LocationCallback")?;
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
