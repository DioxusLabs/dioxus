mod callback;

use dioxus_mobile_core::android::{
    new_object_array, new_string, set_object_array_element, with_activity,
};
use jni::objects::JValue;

/// Request location permission at runtime
pub fn request_permission() -> bool {
    with_activity(|env, activity| {
        if matches!(env.exception_check(), Ok(true)) {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }

        let mut permission_strings = Vec::new();

        #[cfg(feature = "location-coarse")]
        {
            let coarse = new_string(env, "android.permission.ACCESS_COARSE_LOCATION").ok()?;
            permission_strings.push(coarse);
        }

        #[cfg(feature = "location-fine")]
        {
            let fine = new_string(env, "android.permission.ACCESS_FINE_LOCATION").ok()?;
            permission_strings.push(fine);
        }

        #[cfg(feature = "background-location")]
        {
            let background =
                new_string(env, "android.permission.ACCESS_BACKGROUND_LOCATION").ok()?;
            permission_strings.push(background);
        }

        if permission_strings.is_empty() {
            // No static permissions requested, nothing to do (shouldn't happen if feature flags are set)
            return Some(false);
        }

        let permissions_array =
            new_object_array(env, permission_strings.len() as i32, "java/lang/String").ok()?;

        for (index, permission) in permission_strings.into_iter().enumerate() {
            set_object_array_element(env, &permissions_array, index as i32, permission).ok()?;
        }

        const REQUEST_CODE: i32 = 3;
        let helper_class = match callback::load_permissions_helper_class(env) {
            Ok(class) => class,
            Err(_) => {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
                return Some(false);
            }
        };

        if env
            .call_static_method(
                helper_class,
                "requestPermissionsOnUiThread",
                "(Landroid/app/Activity;[Ljava/lang/String;I)V",
                &[
                    JValue::Object(activity),
                    JValue::Object(&permissions_array),
                    JValue::Int(REQUEST_CODE),
                ],
            )
            .is_err()
        {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
            return Some(false);
        }

        Some(true)
    })
    .unwrap_or(false)
}

/// Get the last known location
pub fn last_known() -> Option<(f64, f64)> {
    with_activity(|env, activity| {
        let service_name = new_string(env, "location").ok()?;
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

        let provider = new_string(env, "gps").ok()?;
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
            let fused_provider = new_string(env, "fused").ok()?;
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
