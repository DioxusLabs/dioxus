use jni::{
    objects::{GlobalRef, JClass, JObject, JValue},
    sys::jlong,
    JNIEnv, NativeMethod,
};

use crate::android::java::Result;

/// Generic callback system for loading DEX classes and registering native methods
pub struct CallbackSystem {
    bytecode: &'static [u8],
    class_name: &'static str,
    callback_name: &'static str,
    callback_signature: &'static str,
}

impl CallbackSystem {
    /// Create a new callback system
    ///
    /// # Arguments
    ///
    /// * `bytecode` - The compiled DEX bytecode
    /// * `class_name` - The fully qualified Java class name
    /// * `callback_name` - The name of the native callback method
    /// * `callback_signature` - The JNI signature of the callback method
    pub fn new(
        bytecode: &'static [u8],
        class_name: &'static str,
        callback_name: &'static str,
        callback_signature: &'static str,
    ) -> Self {
        Self {
            bytecode,
            class_name,
            callback_name,
            callback_signature,
        }
    }

    /// Load the DEX class and register the native callback method
    ///
    /// This function handles the boilerplate of:
    /// 1. Creating an InMemoryDexClassLoader
    /// 2. Loading the specified class
    /// 3. Registering the native callback method
    ///
    /// # Returns
    ///
    /// Returns a `GlobalRef` to the loaded class, or an error if loading fails
    pub fn load_and_register(&self, env: &mut JNIEnv<'_>) -> Result<GlobalRef> {
        let callback_class = self.load_dex_class(env)?;
        self.register_native_callback(env, &callback_class)?;
        let global = env.new_global_ref(callback_class)?;
        Ok(global)
    }

    /// Load the DEX class from bytecode
    fn load_dex_class<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JClass<'a>> {
        const IN_MEMORY_LOADER: &str = "dalvik/system/InMemoryDexClassLoader";

        // Create a ByteBuffer from our DEX bytecode
        let byte_buffer = unsafe {
            env.new_direct_byte_buffer(self.bytecode.as_ptr() as *mut u8, self.bytecode.len())
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

        // Load our class
        let class_name = env.new_string(self.class_name)?;
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

    /// Register the native callback method with the Java class
    fn register_native_callback<'a>(
        &self,
        env: &mut JNIEnv<'a>,
        callback_class: &JClass<'a>,
    ) -> Result<()> {
        env.register_native_methods(
            callback_class,
            &[NativeMethod {
                name: self.callback_name.into(),
                sig: self.callback_signature.into(),
                fn_ptr: rust_callback as *mut _,
            }],
        )?;
        Ok(())
    }
}

/// Generic native callback function called from Java
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

    // Use the local reference for this JNI frame; avoid leaking a global ref.
    callback(&mut env, location);
}
