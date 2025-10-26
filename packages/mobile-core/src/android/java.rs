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
