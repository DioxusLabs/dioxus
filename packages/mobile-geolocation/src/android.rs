use dioxus_platform_bridge::android::{
    check_self_permission, load_class_from_classloader, new_object_array, new_string,
    request_permissions_via_helper, set_object_array_element, with_activity,
};
use jni::{
    objects::{JObject, JValue},
    JNIEnv,
};

const PERMISSION_GRANTED: i32 = 0;

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
        let helper_class = match load_class_from_classloader(env, "dioxus.mobile.geolocation.PermissionsHelper") {
            Ok(class) => class,
            Err(_) => {
                let _ = env.exception_describe();
                let _ = env.exception_clear();
                return Some(false);
            }
        };

        if request_permissions_via_helper(env, &helper_class, activity, permissions_array, REQUEST_CODE).is_err() {
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
        // Check permission inline to avoid lifetime issues
        let mut has_permission = false;
        
        #[cfg(feature = "location-fine")]
        {
            has_permission |= check_self_permission(env, activity, "android.permission.ACCESS_FINE_LOCATION").unwrap_or(false);
        }
        
        #[cfg(feature = "location-coarse")]
        {
            has_permission |= check_self_permission(env, activity, "android.permission.ACCESS_COARSE_LOCATION").unwrap_or(false);
        }

        #[cfg(not(any(feature = "location-fine", feature = "location-coarse")))]
        {
            has_permission = true;
        }

        if !has_permission {
            return None;
        }

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
        let mut location = get_last_known_location(env, &location_manager, &provider)?;

        if location.is_null() {
            let fused_provider = new_string(env, "fused").ok()?;
            location = get_last_known_location(env, &location_manager, &fused_provider)?;
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

fn get_last_known_location<'env>(
    env: &mut JNIEnv<'env>,
    manager: &JObject<'env>,
    provider: &JObject<'env>,
) -> Option<JObject<'env>> {
    match env.call_method(
        manager,
        "getLastKnownLocation",
        "(Ljava/lang/String;)Landroid/location/Location;",
        &[JValue::Object(provider)],
    ) {
        Ok(value) => value.l().ok(),
        Err(_) => {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
            None
        }
    }
}

