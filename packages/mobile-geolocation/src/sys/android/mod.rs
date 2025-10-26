mod callback;

use dioxus_mobile_core::android::{
    call_method, call_static_method, find_class, new_object_array, new_string,
    set_object_array_element, with_activity,
};
use jni::{
    objects::{JObject, JValue},
    JNIEnv,
};

/// Request location permission at runtime
pub fn request_permission() -> bool {
    with_activity(|env, activity| {
        let permission = new_string(env, "android.permission.ACCESS_FINE_LOCATION").ok()?;
        let permissions_array = new_object_array(env, 1, "java/lang/String").ok()?;
        set_object_array_element(env, &permissions_array, 0, permission).ok()?;

        const REQUEST_CODE: i32 = 3;
        let activity_class = find_class(env, "androidx/core/app/ActivityCompat").ok()?;

        call_static_method(
            env,
            &activity_class,
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
        let service_name = new_string(env, "location").ok()?;
        let location_manager = call_method(
            env,
            activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&service_name)],
        )
        .ok()?
        .l()
        .ok()?;

        let provider = new_string(env, "gps").ok()?;
        let mut location = call_method(
            env,
            &location_manager,
            "getLastKnownLocation",
            "(Ljava/lang/String;)Landroid/location/Location;",
            &[JValue::Object(&provider)],
        )
        .ok()?
        .l()
        .ok()?;

        if location.is_null() {
            let fused_provider = new_string(env, "fused").ok()?;
            location = call_method(
                env,
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

        let latitude = call_method(&location, "getLatitude", "()D", &[])
            .ok()?
            .d()
            .ok()?;
        let longitude = call_method(&location, "getLongitude", "()D", &[])
            .ok()?
            .d()
            .ok()?;

        Some((latitude, longitude))
    })
}
