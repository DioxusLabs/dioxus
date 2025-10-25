use std::{marker::PhantomData, sync::OnceLock};

use jni::{
    objects::{GlobalRef, JClass, JObject},
    sys::jlong,
    JNIEnv, NativeMethod,
};

use crate::{android::Location, Error, Result};

// NOTE: This must be kept in sync with `LocationCallback.java`.
const RUST_CALLBACK_NAME: &str = "rustCallback";
// NOTE: This must be kept in sync with the signature of `rust_callback`, and
// the signature specified in `LocationCallback.java`.
const RUST_CALLBACK_SIGNATURE: &str = "(JJLandroid/location/Location;)V";

// NOTE: The signature of this function must be kept in sync with
// `RUST_CALLBACK_SIGNATURE`.
unsafe extern "C" fn rust_callback<'a>(
    env: JNIEnv<'a>,
    _: JObject<'a>,
    handler_ptr_high: jlong,
    handler_ptr_low: jlong,
    location: JObject<'a>,
) {
    // TODO: 32-bit? What's that?
    #[cfg(not(target_pointer_width = "64"))]
    compile_error!("non-64-bit Android targets are not supported");

    let handler_ptr: *const super::InnerHandler =
        unsafe { std::mem::transmute([handler_ptr_high, handler_ptr_low]) };
    // SAFETY: See `Drop` implementation for `Manager`.
    let handler = unsafe { &*handler_ptr };

    if let Ok(mut handler) = handler.lock() {
        let location = Location {
            inner: env.new_global_ref(location).unwrap(),
            phantom: PhantomData,
        };
        handler(location);
    }
}

static CALLBACK_CLASS: OnceLock<GlobalRef> = OnceLock::new();

pub(super) fn get_callback_class() -> Result<GlobalRef> {
    if let Some(class) = CALLBACK_CLASS.get() {
        return Ok(class.clone());
    }
    
    // Get JNI environment from ndk_context
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|_| Error::Unknown)?;
    let mut env = vm.attach_current_thread()
        .map_err(|_| Error::Unknown)?;
    
    // Standard JNI class lookup (Gradle will have compiled it)
    let callback_class = env.find_class("com/dioxus/geoloc/LocationCallback")
        .map_err(|_| Error::Unknown)?;
    register_rust_callback(&mut env, &callback_class)?;
    let global = env.new_global_ref(callback_class)
        .map_err(|_| Error::Unknown)?;
    
    Ok(CALLBACK_CLASS.get_or_init(|| global).clone())
}

fn register_rust_callback<'a>(env: &mut JNIEnv<'a>, callback_class: &JClass<'a>) -> Result<()> {
    env.register_native_methods(
        callback_class,
        &[NativeMethod {
            name: RUST_CALLBACK_NAME.into(),
            sig: RUST_CALLBACK_SIGNATURE.into(),
            fn_ptr: rust_callback as *mut _,
        }],
    )
    .map_err(|e| e.into())
}


