mod callback;

use jni::{
    objects::{GlobalRef, JObject, JValue},
    JNIEnv,
};

/// Request location permission at runtime
pub fn request_permission() -> bool {
    let ctx = ndk_context::android_context();
    let vm = match unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) } {
        Ok(vm) => vm,
        Err(_) => return false,
    };
    
    let mut env = match vm.attach_current_thread() {
        Ok(env) => env,
        Err(_) => return false,
    };
    
    let context = ndk_context::android_context().context();
    let context_obj = unsafe { JObject::from_raw(context as jni::sys::jobject) };

    // Request ACCESS_FINE_LOCATION permission
    let permissions = match env.new_string("android.permission.ACCESS_FINE_LOCATION") {
        Ok(p) => p,
        Err(_) => return false,
    };
    
    let array = match env.new_object_array(1, "java/lang/String", &permissions) {
        Ok(a) => a,
        Err(_) => return false,
    };

    // Request code (arbitrary number)
    const REQUEST_CODE: i32 = 3;

    env.call_method(
        &context_obj,
        "requestPermissions",
        "([Ljava/lang/String;I)V",
        &[JValue::Object(&array), JValue::Int(REQUEST_CODE)],
    )
    .is_ok()
}

/// Get the last known location
pub fn last_known() -> Option<(f64, f64)> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    
    let context = ndk_context::android_context().context();
    let context_obj = unsafe { JObject::from_raw(context as jni::sys::jobject) };

    // Get LocationManager service
    let service_name = env.new_string("location").ok()?;
    let location_manager = env
        .call_method(
            &context_obj,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&service_name)],
        )
        .ok()?
        .l()
        .ok()?;

    // Get last known location from GPS provider
    let provider = env.new_string("gps").ok()?;
    let location = env
        .call_method(
            &location_manager,
            "getLastKnownLocation",
            "(Ljava/lang/String;)Landroid/location/Location;",
            &[JValue::Object(&provider)],
        )
        .ok()?
        .l()
        .ok()?;

    // If GPS provider returns null, try fused provider
    let location = if location.is_null() {
        let fused_provider = env.new_string("fused").ok()?;
        env.call_method(
            &location_manager,
            "getLastKnownLocation",
            "(Ljava/lang/String;)Landroid/location/Location;",
            &[JValue::Object(&fused_provider)],
        )
        .ok()?
        .l()
        .ok()?
    } else {
        location
    };

    if location.is_null() {
        return None;
    }

    // Extract latitude and longitude
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
}
