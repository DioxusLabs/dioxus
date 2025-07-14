pub(crate) fn cross_open(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "ios")]
    {
        return open_url_ios(url);
    }

    #[cfg(target_os = "android")]
    {
        return open_url_android(url);
    }

    open::that(url).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(target_os = "ios")]
fn open_url_ios(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};
    use std::ffi::CString;

    unsafe {
        // Get UIApplication class
        let ui_app_class = Class::get("UIApplication").unwrap();

        // Get shared application instance
        let shared_app: *mut Object = msg_send![ui_app_class, sharedApplication];

        // Create NSString from URL
        let ns_string_class = Class::get("NSString").unwrap();
        let url_cstring = CString::new(url)?;
        let ns_url_string: *mut Object = msg_send![ns_string_class,
            stringWithUTF8String: url_cstring.as_ptr()];

        // Create NSURL from NSString
        let nsurl_class = Class::get("NSURL").unwrap();
        let nsurl: *mut Object = msg_send![nsurl_class, URLWithString: ns_url_string];

        // Check if URL is valid
        if nsurl.is_null() {
            return Err("Invalid URL".into());
        }

        // Check if URL can be opened
        let can_open: bool = msg_send![shared_app, canOpenURL: nsurl];

        if can_open {
            // Open the URL (iOS 10+ style with completion handler)
            let _: () = msg_send![shared_app, openURL: nsurl
                                 options: std::ptr::null::<Object>()
                                 completionHandler: std::ptr::null::<Object>()];
        } else {
            return Err("Cannot open URL".into());
        }
    }

    Ok(())
}

#[cfg(target_os = "android")]
fn open_url_android(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    use jni::objects::JObject;
    use jni::objects::JValue;
    use std::ptr::NonNull;

    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
    let mut env = vm.attach_current_thread().unwrap();

    // Get the activity context
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    // Create Intent with ACTION_VIEW
    let intent_class = env.find_class("android/content/Intent")?;
    let intent = env.new_object(intent_class, "()V", &[])?;

    // Set action to ACTION_VIEW
    let action_view = env.new_string("android.intent.action.VIEW")?;
    env.call_method(
        &intent,
        "setAction",
        "(Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&JObject::from(action_view))],
    )?;

    // Create Uri from URL string
    let uri_class = env.find_class("android/net/Uri")?;
    let url_string = env.new_string(url)?;
    let uri = env.call_static_method(
        uri_class,
        "parse",
        "(Ljava/lang/String;)Landroid/net/Uri;",
        &[JValue::Object(&JObject::from(url_string))],
    )?;

    // Set data URI on intent
    env.call_method(
        &intent,
        "setData",
        "(Landroid/net/Uri;)Landroid/content/Intent;",
        &[(&uri).into()],
    )?;

    // Start activity
    env.call_method(
        activity,
        "startActivity",
        "(Landroid/content/Intent;)V",
        &[JValue::Object(&intent)],
    )?;

    Ok(())
}
