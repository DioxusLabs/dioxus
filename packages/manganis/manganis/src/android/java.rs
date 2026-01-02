use jni::{
    objects::{JClass, JObject, JObjectArray, JString, JValue, JValueOwned},
    JNIEnv,
};

/// Result type for JNI operations
pub type Result<T> = std::result::Result<T, jni::errors::Error>;

/// Helper functions for common JNI operations

/// Create a new Java string tied to the current JNI frame
pub fn new_string<'env>(env: &mut JNIEnv<'env>, s: &str) -> Result<JString<'env>> {
    env.new_string(s)
}

/// Create a new object array
pub fn new_object_array<'env>(
    env: &mut JNIEnv<'env>,
    len: i32,
    element_class: &str,
) -> Result<JObjectArray<'env>> {
    env.new_object_array(len, element_class, &JObject::null())
}

/// Set an element in an object array
pub fn set_object_array_element<'env>(
    env: &mut JNIEnv<'env>,
    array: &JObjectArray<'env>,
    index: i32,
    element: JString<'env>,
) -> Result<()> {
    env.set_object_array_element(array, index, element)
}

/// Call a static method on a class
pub fn call_static_method<'env, 'obj>(
    env: &mut JNIEnv<'env>,
    class: &JClass<'env>,
    method_name: &str,
    signature: &str,
    args: &[JValue<'env, 'obj>],
) -> Result<JValueOwned<'env>> {
    env.call_static_method(class, method_name, signature, args)
}

/// Call an instance method on an object
pub fn call_method<'env, 'obj>(
    env: &mut JNIEnv<'env>,
    obj: &JObject<'env>,
    method_name: &str,
    signature: &str,
    args: &[JValue<'env, 'obj>],
) -> Result<JValueOwned<'env>> {
    env.call_method(obj, method_name, signature, args)
}

/// Find a Java class by name
pub fn find_class<'env>(env: &mut JNIEnv<'env>, class_name: &str) -> Result<JClass<'env>> {
    env.find_class(class_name)
}

/// Create a new object instance
pub fn new_object<'env, 'obj>(
    env: &mut JNIEnv<'env>,
    class_name: &str,
    signature: &str,
    args: &[JValue<'env, 'obj>],
) -> Result<JObject<'env>> {
    env.new_object(class_name, signature, args)
}

/// Check if a permission is granted (Activity.checkSelfPermission)
pub fn check_self_permission(
    env: &mut JNIEnv,
    activity: &JObject,
    permission: &str,
) -> Result<bool> {
    let permission_string = new_string(env, permission)?;
    let status = env.call_method(
        activity,
        "checkSelfPermission",
        "(Ljava/lang/String;)I",
        &[JValue::Object(&permission_string)],
    )?;

    const PERMISSION_GRANTED: i32 = 0;
    Ok(status.i()? == PERMISSION_GRANTED)
}

/// Request permissions via a helper class's static method
///
/// This uses PermissionsHelper.requestPermissionsOnUiThread(pattern)
/// to request permissions on the UI thread.
pub fn request_permissions_via_helper(
    env: &mut JNIEnv,
    helper_class: &JClass,
    activity: &JObject,
    permissions: JObjectArray,
    request_code: i32,
) -> Result<()> {
    env.call_static_method(
        helper_class,
        "requestPermissionsOnUiThread",
        "(Landroid/app/Activity;[Ljava/lang/String;I)V",
        &[
            JValue::Object(activity),
            JValue::Object(&permissions.into()),
            JValue::Int(request_code),
        ],
    )?;
    Ok(())
}

/// Load a Java class from the APK's classloader
pub fn load_class_from_classloader<'env>(
    env: &mut JNIEnv<'env>,
    class_name: &str,
) -> Result<JClass<'env>> {
    let class_name_jstring = new_string(env, class_name)?;
    let class = env.call_static_method(
        "java/lang/Class",
        "forName",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&class_name_jstring)],
    )?;
    Ok(class.l()?.into())
}
