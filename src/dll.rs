use std::ffi::c_void;

use crate::{
    properties::JXLPropertyStore,
    registry::{register, unregister},
    JXLWICBitmapDecoder,
};
use windows as Windows;
use windows::runtime::{implement, IUnknown, Interface, GUID, HRESULT};
use windows::Win32::{
    Foundation::*, System::LibraryLoader::GetModuleFileNameW,
    System::SystemServices::DLL_PROCESS_ATTACH,
};

static mut DLL_INSTANCE: HINSTANCE = HINSTANCE { 0: 0 };

fn get_module_path(instance: HINSTANCE) -> Result<String, HRESULT> {
    let mut path: Vec<u16> = Vec::new();
    path.reserve(1024);
    let path_len = unsafe {
        GetModuleFileNameW(
            instance,
            std::mem::transmute(path.as_mut_ptr()),
            path.capacity() as u32,
        )
    };

    let path_len = path_len as usize;
    if path_len == 0 || path_len >= path.capacity() {
        return Err(E_FAIL);
    }
    unsafe {
        path.set_len(path_len + 1);
    }
    String::from_utf16(&path).map_err(|_| E_FAIL)
}

#[implement(Windows::Win32::System::Com::IClassFactory)]
struct ClassFactory {}

#[allow(non_snake_case)]
impl ClassFactory {
    pub unsafe fn CreateInstance(
        &self,
        outer: &Option<windows::runtime::IUnknown>,
        iid: *const GUID,
        object: *mut windows::runtime::RawPtr,
    ) -> HRESULT {
        if outer.is_some() {
            return CLASS_E_NOAGGREGATION;
        }
        match *iid {
            windows::Win32::Graphics::Imaging::IWICBitmapDecoder::IID => {
                let unknown: IUnknown = JXLWICBitmapDecoder::default().into();
                unknown.query(iid, object)
            }
            windows::Win32::System::PropertiesSystem::IPropertyStore::IID => {
                let unknown: IUnknown = JXLPropertyStore::default().into();
                unknown.query(iid, object)
            }
            _ => {
                log::trace!("Unknown IID: {:?}", *iid);
                E_NOINTERFACE
            }
        }
    }
    pub unsafe fn LockServer(&self, _flock: BOOL) -> windows::runtime::Result<()> {
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
    pout: *mut windows::runtime::RawPtr,
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
        JXLWICBitmapDecoder::CLSID | JXLPropertyStore::CLSID => unknown.query(riid, pout),
        _ => CLASS_E_CLASSNOTAVAILABLE,
    }
}
