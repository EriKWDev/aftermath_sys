pub use aftermath_sys as sys;

pub struct Aftermath {
    pub user_data_ptr: *mut core::ffi::c_void,
}

impl Default for Aftermath {
    fn default() -> Self {
        Self::new(DefaultAftermathCallbacks)
    }
}

unsafe impl Send for Aftermath {}
unsafe impl Sync for Aftermath {}

impl Aftermath {
    pub fn new<T: AftermathCallbacks>(callbacks: T) -> Self {
        let slf = Box::into_raw(Box::new(callbacks)) as *mut core::ffi::c_void;
        let user_data = Box::new(AftermathCallbackFunctions {
            slf,
            dumped: trampoline_dumped::<T>,
            shader_debug_info: trampoline_shader_debug_info::<T>,
            description: trampoline_description::<T>,
            drop: trampoline_drop::<T>,
        });
        let user_data_ptr = Box::into_raw(user_data) as *mut core::ffi::c_void;

        unsafe {
            sys::GFSDK_Aftermath_EnableGpuCrashDumps(
                sys::GFSDK_Aftermath_Version_GFSDK_Aftermath_Version_API,
                sys::GFSDK_Aftermath_GpuCrashDumpWatchedApiFlags_GFSDK_Aftermath_GpuCrashDumpWatchedApiFlags_Vulkan,
                sys::GFSDK_Aftermath_GpuCrashDumpFeatureFlags_GFSDK_Aftermath_GpuCrashDumpFeatureFlags_Default,
                Some(GpuCrashDumpCallback),
                Some(ShaderDebugInfoCallback),
                Some(GpuCrashDumpDescriptionCallback),
                Some(ResolveMarkerCallback),
                user_data_ptr,
            );
        }

        Self { user_data_ptr }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    NotStarted = 0,
    CollectingData,
    CollectingDataFailed,
    InvokingCallback,
    Finished,
    Unknown,
}

impl Status {
    pub fn get() -> Self {
        let mut status: u32 = 0;
        unsafe {
            sys::GFSDK_Aftermath_GetCrashDumpStatus(&mut status);
            let status = status as u8;
            std::mem::transmute(status)
        }
    }

    pub fn wait_for_status(timeout: Option<std::time::Duration>) -> Self {
        let mut status = Self::get();
        let delta = core::time::Duration::from_millis(50);
        let mut time = core::time::Duration::new(0, 0);

        while status != Status::CollectingDataFailed
            && status != Status::Finished
            && timeout.map_or(true, |t| time < t)
        {
            std::thread::sleep(delta);
            time += delta;
            status = Self::get();
        }

        status
    }
}

impl Drop for Aftermath {
    fn drop(&mut self) {
        let functions =
            unsafe { Box::from_raw(self.user_data_ptr as *mut AftermathCallbackFunctions) };
        (functions.drop)(functions.slf);
        drop(functions);
    }
}

pub trait AftermathCallbacks: Send + Sync + 'static {
    fn dumped(&mut self, dump_data: &[u8]);
    fn shader_debug_info(&mut self, data: &[u8]);
    fn description(&mut self, describe: &mut DescriptionBuilder);
}

#[doc(hidden)]
pub fn default_call_num_and_output_path() -> (usize, std::path::PathBuf) {
    thread_local! {
        pub static DUMP_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    }

    let dump_n = DUMP_N.with(|it| it.fetch_add(1, std::sync::atomic::Ordering::Relaxed));

    let mut n = 0;
    let base = std::path::PathBuf::from("/tmp");
    let mut path = base.join(format!("aftermath_dump_{n}"));
    if path.exists() {
        loop {
            let next = base.join(format!("aftermath_dump_{}", n + 1));
            if !next.exists() {
                if dump_n == 0 {
                    path = next;
                }
                break;
            }
            path = next;
            n += 1;
        }
    }

    let _ = std::fs::create_dir_all(&path);
    let path = path.canonicalize().unwrap();
    (dump_n, path)
}

pub struct DefaultAftermathCallbacks;
impl AftermathCallbacks for DefaultAftermathCallbacks {
    fn dumped(&mut self, dump_data: &[u8]) {
        let (n, path) = default_call_num_and_output_path();
        let path = path.join(format!("{n}_gpu_crash_dump.nv-gpudmp"));

        if let Err(err) = std::fs::write(&path, dump_data) {
            eprintln!(
                "Could not write NVIDIA Aftermath crash dump to '{}': {err}",
                path.display()
            );
            return;
        }

        println!("Wrote NVIDIA Aftermath crash dump to '{}'", path.display());
    }

    fn shader_debug_info(&mut self, data: &[u8]) {
        let (n, path) = default_call_num_and_output_path();
        let path = path.join(format!("{n}_shader_debug_info.nv-shader-debug"));

        if let Err(err) = std::fs::write(&path, data) {
            eprintln!(
                "Could not write NVIDIA Aftermath shader debug info to '{}': {err}",
                path.display()
            );
            return;
        }

        println!(
            "Wrote NVIDIA Aftermath shader debug info to '{}'",
            path.display()
        );
    }

    fn description(&mut self, describe: &mut DescriptionBuilder) {
        use std::str::FromStr;

        let name = env!("CARGO_PKG_NAME");
        let version = format!("{name}-{}-sys-{}", env!("CARGO_PKG_VERSION"), sys::VERSION);

        let name_c = std::ffi::CString::from_str(&name).unwrap();
        describe.set_application_name(&name_c);

        let version_c = std::ffi::CString::from_str(&version).unwrap();
        describe.set_application_version(&version_c);
    }
}

#[doc(hidden)]
pub struct AftermathCallbackFunctions {
    pub slf: *mut core::ffi::c_void,
    pub dumped: fn(slf: *mut core::ffi::c_void, dump_data: &[u8]),
    pub shader_debug_info: fn(slf: *mut core::ffi::c_void, data: &[u8]),
    pub description: fn(slf: *mut core::ffi::c_void, describe: &mut DescriptionBuilder),
    pub drop: fn(*mut core::ffi::c_void),
}

#[doc(hidden)]
pub fn trampoline_dumped<T: AftermathCallbacks>(slf: *mut core::ffi::c_void, dump_data: &[u8]) {
    let mut slf = unsafe { Box::from_raw(slf as *mut T) };
    slf.dumped(dump_data);
    let _ = Box::into_raw(slf);
}
#[doc(hidden)]
pub fn trampoline_shader_debug_info<T: AftermathCallbacks>(
    slf: *mut core::ffi::c_void,
    data: &[u8],
) {
    let mut slf = unsafe { Box::from_raw(slf as *mut T) };
    slf.shader_debug_info(data);
    let _ = Box::into_raw(slf);
}
#[doc(hidden)]
pub fn trampoline_description<T: AftermathCallbacks>(
    slf: *mut core::ffi::c_void,
    describe: &mut DescriptionBuilder,
) {
    let mut slf = unsafe { Box::from_raw(slf as *mut T) };
    slf.description(describe);
    let _ = Box::into_raw(slf);
}
#[doc(hidden)]
pub fn trampoline_drop<T: AftermathCallbacks>(slf: *mut core::ffi::c_void) {
    let slf = unsafe { Box::from_raw(slf as *mut T) };
    drop(slf);
}

#[doc(hidden)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn GpuCrashDumpCallback(
    pGpuCrashDump: *const ::core::ffi::c_void,
    gpuCrashDumpSize: u32,
    pUserData: *mut ::core::ffi::c_void,
) {
    let functions = unsafe { Box::from_raw(pUserData as *mut AftermathCallbackFunctions) };
    let data = unsafe {
        core::slice::from_raw_parts(pGpuCrashDump as *const u8, gpuCrashDumpSize as usize)
    };
    (functions.dumped)(functions.slf, data);
    let _ = Box::into_raw(functions);
}

#[doc(hidden)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn ShaderDebugInfoCallback(
    pData: *const ::core::ffi::c_void,
    size: u32,
    pUserData: *mut ::core::ffi::c_void,
) {
    let functions = unsafe { Box::from_raw(pUserData as *mut AftermathCallbackFunctions) };
    let data = unsafe { core::slice::from_raw_parts(pData as *const u8, size as usize) };
    (functions.shader_debug_info)(functions.slf, data);
    let _ = Box::into_raw(functions);
}

#[doc(hidden)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn GpuCrashDumpDescriptionCallback(
    callback: sys::PFN_GFSDK_Aftermath_AddGpuCrashDumpDescription,
    pUserData: *mut ::core::ffi::c_void,
) {
    let functions = unsafe { Box::from_raw(pUserData as *mut AftermathCallbackFunctions) };
    let mut builder = DescriptionBuilder(callback);
    (functions.description)(functions.slf, &mut builder);
    let _ = Box::into_raw(functions);
}

#[doc(hidden)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn ResolveMarkerCallback(
    _pMarkerData: *const ::core::ffi::c_void,
    _markerDataSize: u32,
    _pUserData: *mut ::core::ffi::c_void,
    _resolveMarker: sys::PFN_GFSDK_Aftermath_ResolveMarker,
) {
}

#[repr(C)]
pub struct DescriptionBuilder(sys::PFN_GFSDK_Aftermath_AddGpuCrashDumpDescription);

impl DescriptionBuilder {
    pub fn set_application_name(&mut self, name: &core::ffi::CStr) {
        unsafe {
            (self.0).as_ref().unwrap()(1, name.as_ptr());
        }
    }
    pub fn set_application_version(&mut self, name: &core::ffi::CStr) {
        unsafe {
            (self.0).as_ref().unwrap()(2, name.as_ptr());
        }
    }
    pub fn set(&mut self, index: u32, name: &core::ffi::CStr) {
        unsafe {
            (self.0).as_ref().unwrap()(0x10000 + index, name.as_ptr());
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_usage() {
        let aftermath = Aftermath::new(DefaultAftermathCallbacks);
        drop(aftermath);
    }
}
