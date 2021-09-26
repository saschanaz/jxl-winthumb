use std::ffi::c_void;

use crate::{
    bindings::Windows,
    guid::JXLWINTHUMB_THUMBNAILPROVIDER_CLSID,
    registry::{register_clsid, register_provider, unregister_clsid, unregister_provider},
    wic::JXLWICBitmapDecoder,
};
use windows::{implement, Guid, IUnknown, Interface, HRESULT};
use Windows::{
    Win32::Foundation::{
        BOOL, CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, E_NOINTERFACE, E_NOTIMPL,
        E_UNEXPECTED, HINSTANCE, S_OK,
    },
    Win32::System::LibraryLoader::GetModuleFileNameW,
    Win32::System::SystemServices::DLL_PROCESS_ATTACH,
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
        outer: &Option<windows::IUnknown>,
        iid: *const Guid,
        object: *mut windows::RawPtr,
    ) -> HRESULT {
        if outer.is_some() {
            return CLASS_E_NOAGGREGATION;
        }
        match *iid {
            crate::bindings::Windows::Win32::UI::Shell::IThumbnailProvider::IID => {
                let unknown: IUnknown = crate::ThumbnailProvider::default().into();
                unknown.query(iid, object)
            }
            crate::bindings::Windows::Win32::Graphics::Imaging::IWICBitmapDecoder::IID => {
                let unknown: IUnknown = JXLWICBitmapDecoder::default().into();
                unknown.query(iid, object)
            }
            _ => {
                log::trace!("Unknown IID: {:?}", *iid);
                E_NOINTERFACE
            }
        }
    }
    pub unsafe fn LockServer(&self, _flock: BOOL) -> windows::Result<()> {
        E_NOTIMPL.ok()
    }
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
    if register_clsid(&module_path).is_ok() && register_provider().is_ok() {
        S_OK
    } else {
        E_FAIL
    }
}

#[no_mangle]
#[allow(non_snake_case)]
#[doc(hidden)]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    if unregister_clsid().is_ok() && unregister_provider().is_ok() {
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
    rclsid: *const Guid,
    riid: *const Guid,
    pout: *mut windows::RawPtr,
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
    if *riid != crate::bindings::Windows::Win32::System::Com::IClassFactory::IID {
        return E_UNEXPECTED;
    }

    let factory = ClassFactory {};
    let unknown: IUnknown = factory.into();

    match *rclsid {
        JXLWINTHUMB_THUMBNAILPROVIDER_CLSID | JXLWICBitmapDecoder::CLSID => {
            unknown.query(riid, pout)
        }
        _ => CLASS_E_CLASSNOTAVAILABLE,
    }
}
