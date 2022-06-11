use std::ffi::c_void;

use crate::{
    properties::JXLPropertyStore,
    registry::{register, unregister},
    JXLWICBitmapDecoder,
};
use windows as Windows;
use windows::core::{implement, IUnknown, Interface, GUID, HRESULT};
use windows::Win32::{
    Foundation::*, System::Com::IClassFactory_Impl, System::LibraryLoader::GetModuleFileNameW,
    System::SystemServices::DLL_PROCESS_ATTACH,
};

static mut DLL_INSTANCE: HINSTANCE = HINSTANCE(0);

fn get_module_path(instance: HINSTANCE) -> Result<String, HRESULT> {
    let mut path = [0u16; MAX_PATH as usize];
    let path_len = unsafe { GetModuleFileNameW(instance, &mut path) } as usize;
    String::from_utf16(&path[0..path_len]).map_err(|_| E_FAIL)
}

#[implement(Windows::Win32::System::Com::IClassFactory)]
struct ClassFactory {}

impl IClassFactory_Impl for ClassFactory {
    fn CreateInstance(
        &self,
        outer: &Option<windows::core::IUnknown>,
        iid: *const GUID,
        object: *mut windows::core::RawPtr,
    ) -> windows::core::Result<()> {
        if outer.is_some() {
            return CLASS_E_NOAGGREGATION.ok();
        }
        unsafe {
            match *iid {
                windows::Win32::Graphics::Imaging::IWICBitmapDecoder::IID => {
                    let unknown: IUnknown = JXLWICBitmapDecoder::default().into();
                    unknown.query(&*iid, object as _).ok()
                }
                windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore::IID => {
                    let unknown: IUnknown = JXLPropertyStore::default().into();
                    unknown.query(&*iid, object as _).ok()
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
    use std::ptr::null_mut;
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, null_mut(), null_mut()) };
}

#[no_mangle]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    let module_path = {
        let result = get_module_path(DLL_INSTANCE);
        if let Err(err) = result {
            return err;
        }
        result.unwrap()
    };
    if register(&module_path).is_ok() {
        shell_change_notify();
        S_OK
    } else {
        E_FAIL
    }
}

#[no_mangle]
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

#[no_mangle]
#[allow(non_snake_case)]
#[doc(hidden)]
pub extern "stdcall" fn DllMain(
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

#[no_mangle]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    pout: *mut *const core::ffi::c_void,
) -> HRESULT {
    // Sets up logging to the Cargo.toml directory for debug purposes.
    #[cfg(debug_assertions)]
    {
        // Set up logging to the project directory.
        simple_logging::log_to_file(
            &format!("{}\\debug.log", env!("CARGO_MANIFEST_DIR")),
            log::LevelFilter::Trace,
        )
        .unwrap();
    }
    log::trace!("DllGetClassObject");
    if *riid != windows::Win32::System::Com::IClassFactory::IID {
        return E_UNEXPECTED;
    }

    let factory = ClassFactory {};
    let unknown: IUnknown = factory.into();

    match *rclsid {
        JXLWICBitmapDecoder::CLSID | JXLPropertyStore::CLSID => unknown.query(&*riid, pout),
        _ => CLASS_E_CLASSNOTAVAILABLE,
    }
}
