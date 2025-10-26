use jni::{
    objects::{JObject, JValue},
    JNIEnv,
};

/// Result type for JNI operations
pub type Result<T> = std::result::Result<T, jni::errors::Error>;

/// Helper functions for common JNI operations

/// Create a new Java string
pub fn new_string(env: &mut JNIEnv<'_>, s: &str) -> Result<jni::objects::JString<'_>> {
    env.new_string(s)
}

/// Create a new object array
pub fn new_object_array(
    env: &mut JNIEnv<'_>,
    len: i32,
    element_class: &str,
) -> Result<jni::objects::JObjectArray<'_>> {
    env.new_object_array(len, element_class, &JObject::null())
}

/// Set an element in an object array
pub fn set_object_array_element(
    env: &mut JNIEnv<'_>,
    array: &jni::objects::JObjectArray<'_>,
    index: i32,
    element: jni::objects::JString<'_>,
) -> Result<()> {
    env.set_object_array_element(array, index, element)
}

/// Call a static method on a class
pub fn call_static_method(
    env: &mut JNIEnv<'_>,
    class: &jni::objects::JClass<'_>,
    method_name: &str,
    signature: &str,
    args: &[JValue<'_>],
) -> Result<jni::objects::JValue<'_>> {
    env.call_static_method(class, method_name, signature, args)
}

/// Call an instance method on an object
pub fn call_method(
    env: &mut JNIEnv<'_>,
    obj: &JObject<'_>,
    method_name: &str,
    signature: &str,
    args: &[JValue<'_>],
) -> Result<jni::objects::JValue<'_>> {
    env.call_method(obj, method_name, signature, args)
}

/// Find a Java class by name
pub fn find_class(env: &mut JNIEnv<'_>, class_name: &str) -> Result<jni::objects::JClass<'_>> {
    env.find_class(class_name)
}

/// Create a new object instance
pub fn new_object(
    env: &mut JNIEnv<'_>,
    class_name: &str,
    signature: &str,
    args: &[JValue<'_>],
) -> Result<jni::objects::JObject<'_>> {
    env.new_object(class_name, signature, args)
}
