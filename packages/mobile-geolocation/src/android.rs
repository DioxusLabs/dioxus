//! Android geolocation implementation via JNI and Java shim

pub mod callback;

use std::{
    marker::PhantomData,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use jni::{
    objects::{GlobalRef, JObject, JValueGen},
    JNIEnv,
};

use crate::{Coordinates, Error, Result};

// This will be populated by the LocationCallback class after DEX loading
type InnerHandler = Mutex<dyn FnMut(Location)>;

pub struct Manager {
    callback: GlobalRef,
    // We "leak" the handler so that `rust_callback` can safely access it
    inner: *const InnerHandler,
}

impl Manager {
    pub fn new<F>(handler: F) -> Result<Self>
    where
        F: FnMut(Location) + 'static,
    {
        let inner = Box::into_raw(Box::new(Mutex::new(handler)));

        Ok(Manager {
            callback: callback::get_callback_class()?,
            inner,
        })
    }

    pub fn last_known() -> Result<Location> {
        Err(Error::NotSupported)
    }
}

/// Request location permissions
pub fn request_permission() -> bool {
    use jni::objects::JObject;
    
    // Get JNI environment from ndk_context
    let ctx = ndk_context::android_context();
    let vm = match unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) } {
        Ok(vm) => vm,
        Err(e) => {
            eprintln!("Failed to get JavaVM: {:?}", e);
            return false;
        }
    };
    
    let mut env = match vm.attach_current_thread() {
        Ok(env) => env,
        Err(e) => {
            eprintln!("Failed to attach to current thread: {:?}", e);
            return false;
        }
    };
    
    // Get the Android Activity
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
    
    // Call GeolocationShim.requestPermission() from the Kotlin shim
    let shim_class = match env.find_class("com/dioxus/geoloc/GeolocationShim") {
        Ok(class) => class,
        Err(e) => {
            eprintln!("Failed to find GeolocationShim class: {:?}", e);
            return false;
        }
    };
    
    // Call the static method requestPermission(Activity, int, boolean): void
    match env.call_static_method(
        shim_class,
        "requestPermission",
        "(Landroid/app/Activity;IZ)V",
        &[(&activity).into(), 1000.into(), true.into()], // requestCode=1000, fine=true
    ) {
        Ok(_) => {
            eprintln!("Permission request sent to Android system");
            true
        }
        Err(e) => {
            eprintln!("Failed to request permission: {:?}", e);
            false
        }
    }
}

/// Get the last known location (public API)
pub fn last_known() -> Option<(f64, f64)> {
    use jni::objects::JObject;
    
    // Get JNI environment from ndk_context
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    
    // Get the Android Activity
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
    
    // Call GeolocationShim.lastKnown() from the Kotlin shim
    let shim_class = match env.find_class("com/dioxus/geoloc/GeolocationShim") {
        Ok(class) => class,
        Err(e) => {
            eprintln!("Failed to find GeolocationShim class: {:?}", e);
            return None;
        }
    };
    
    // Call the static method lastKnown(Activity): DoubleArray?
    let result = match env.call_static_method(
        shim_class,
        "lastKnown",
        "(Landroid/app/Activity;)[D",
        &[(&activity).into()],
    ) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to call lastKnown method: {:?}", e);
            return None;
        }
    };
    
    // Get the double array result
    let double_array = match result.l() {
        Ok(array) => array,
        Err(e) => {
            eprintln!("Failed to get array from result: {:?}", e);
            return None;
        }
    };
    
    if double_array.is_null() {
        eprintln!("GeolocationShim.lastKnown() returned null - no location available or permissions denied");
        return None;
    }
    
    // Convert to JDoubleArray
    let array: jni::objects::JDoubleArray = double_array.into();
    
    // Get array length
    let len = match env.get_array_length(&array) {
        Ok(length) => length,
        Err(e) => {
            eprintln!("Failed to get array length: {:?}", e);
            return None;
        }
    };
    
    if len < 2 {
        eprintln!("Array length is less than 2: {}", len);
        return None;
    }
    
    // Get elements from the double array
    let mut buf = vec![0.0; len as usize];
    match env.get_double_array_region(&array, 0, &mut buf) {
        Ok(_) => {
            eprintln!("Successfully retrieved location: lat={}, lon={}", buf[0], buf[1]);
            Some((buf[0], buf[1]))
        }
        Err(e) => {
            eprintln!("Failed to get array elements: {:?}", e);
            None
        }
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        // Stop receiving updates before dropping
        // Note: In a full implementation, we'd call stop_updates here
        
        // SAFETY: We have stopped updates, so nothing else will touch the data behind this pointer
        let _ = unsafe { Box::from_raw(self.inner as *mut InnerHandler) };
    }
}

pub struct Location {
    inner: GlobalRef,
    phantom: PhantomData<()>,
}

impl Location {
    pub fn coordinates(&self) -> Result<Coordinates> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
            .map_err(|_| Error::Unknown)?;
        let mut env = vm.attach_current_thread()
            .map_err(|_| Error::Unknown)?;
        
        let latitude = env
            .call_method(&self.inner, "getLatitude", "()D", &[])?
            .d()?;
        let longitude = env
            .call_method(&self.inner, "getLongitude", "()D", &[])?
            .d()?;
        
        Ok(Coordinates { latitude, longitude })
    }

    pub fn altitude(&self) -> Result<f64> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
            .map_err(|_| Error::Unknown)?;
        let mut env = vm.attach_current_thread()
            .map_err(|_| Error::Unknown)?;
        
        env.call_method(&self.inner, "getAltitude", "()D", &[])?
            .d()
            .map_err(|_| Error::Unknown)
    }

    pub fn bearing(&self) -> Result<f64> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
            .map_err(|_| Error::Unknown)?;
        let mut env = vm.attach_current_thread()
            .map_err(|_| Error::Unknown)?;
        
        match env.call_method(&self.inner, "getBearing", "()F", &[])?.f() {
            Ok(bearing) => Ok(bearing as f64),
            Err(_) => Err(Error::Unknown),
        }
    }

    pub fn speed(&self) -> Result<f64> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
            .map_err(|_| Error::Unknown)?;
        let mut env = vm.attach_current_thread()
            .map_err(|_| Error::Unknown)?;
        
        match env.call_method(&self.inner, "getSpeed", "()F", &[])?.f() {
            Ok(speed) => Ok(speed as f64),
            Err(_) => Err(Error::Unknown),
        }
    }

    pub fn time(&self) -> Result<SystemTime> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
            .map_err(|_| Error::Unknown)?;
        let mut env = vm.attach_current_thread()
            .map_err(|_| Error::Unknown)?;
        
        match env.call_method(&self.inner, "getTime", "()J", &[])?.j() {
            Ok(time_ms) => Ok(SystemTime::UNIX_EPOCH + Duration::from_millis(time_ms.try_into().unwrap_or(0))),
            Err(_) => Err(Error::Unknown),
        }
    }
}

impl From<jni::errors::Error> for Error {
    fn from(_: jni::errors::Error) -> Self {
        Error::Unknown
    }
}
