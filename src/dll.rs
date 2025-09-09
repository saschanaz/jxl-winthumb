use std::ffi::c_void;

use crate::{
    JXLWICBitmapDecoder,
    properties::JXLPropertyStore,
    registry::{register, unregister},
};
use windows as Windows;
use windows::Win32::{
    Foundation::*,
    System::Com::IClassFactory_Impl,
    System::LibraryLoader::GetModuleFileNameW,
    System::SystemServices::DLL_PROCESS_ATTACH,
    UI::Shell::PropertiesSystem::{IInitializeWithStream, IPropertyStore},
};
use windows::core::{GUID, HRESULT, IUnknown, Interface, implement};

static mut DLL_INSTANCE: HINSTANCE = HINSTANCE(std::ptr::null_mut());

fn get_module_path(instance: HINSTANCE) -> Result<String, HRESULT> {
    let mut path = [0u16; MAX_PATH as usize];
    let path_len = unsafe { GetModuleFileNameW(instance, &mut path) } as usize;
    String::from_utf16(&path[0..path_len]).map_err(|_| E_FAIL)
}

#[implement(Windows::Win32::System::Com::IClassFactory)]
struct ClassFactory {}

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Option<&windows::core::IUnknown>,
        iid: *const GUID,
        object: *mut *mut core::ffi::c_void,
    ) -> windows::core::Result<()> {
        if outer.is_some() {
            return CLASS_E_NOAGGREGATION.ok();
        }
        unsafe {
            match *iid {
                windows::Win32::Graphics::Imaging::IWICBitmapDecoder::IID => {
                    let unknown: IUnknown = JXLWICBitmapDecoder::default().into();
                    unknown.query(iid, object).ok()
                }
                IPropertyStore::IID | IInitializeWithStream::IID => {
                    let unknown: IUnknown = JXLPropertyStore::default().into();
                    unknown.query(iid, object).ok()
                }
                _ => {
                    log::trace!("Unknown IID: {:?}", *iid);
                    E_NOINTERFACE.ok()
                }
            }
        }
    }
    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        E_NOTIMPL.ok()
    }
}

fn shell_change_notify() {
    use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    let module_path = match get_module_path(unsafe { DLL_INSTANCE }) {
        Ok(path) => path,
        Err(err) => return err,
    };
    if register(&module_path).is_ok() {
        shell_change_notify();
        S_OK
    } else {
        E_FAIL
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    if unregister().is_ok() {
        shell_change_notify();
        S_OK
    } else {
        E_FAIL
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
#[doc(hidden)]
pub extern "system" fn DllMain(
    dll_instance: HINSTANCE,
    reason: u32,
    _reserved: *mut c_void,
) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        unsafe {
            DLL_INSTANCE = dll_instance;
        }
    }
    true
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    pout: *mut *mut core::ffi::c_void,
) -> HRESULT {
    // Sets up logging to the Cargo.toml directory for debug purposes.
    #[cfg(debug_assertions)]
    {
        // Set up logging to the project directory.
        use std::time::SystemTime;
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
        if duration.is_err() {
            return E_UNEXPECTED;
        }
        let secs = duration.unwrap().as_secs();
        let dir = env!("CARGO_MANIFEST_DIR");
        simple_logging::log_to_file(format!("{dir}\\debug-{secs}.log"), log::LevelFilter::Trace)
            .unwrap();
    }
    log::trace!("DllGetClassObject");
    if unsafe { *riid } != windows::Win32::System::Com::IClassFactory::IID {
        return E_UNEXPECTED;
    }

    let factory = ClassFactory {};
    let unknown: IUnknown = factory.into();

    match unsafe { *rclsid } {
        JXLWICBitmapDecoder::CLSID | JXLPropertyStore::CLSID => unsafe {
            unknown.query(riid, pout)
        },
        _ => CLASS_E_CLASSNOTAVAILABLE,
    }
}
