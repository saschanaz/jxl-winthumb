use std::ptr::null_mut;

use windows::Interface;
use winreg::enums::*;
use winreg::RegKey;

use crate::bindings::Windows::Win32::UI::Shell::{
    SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
};
use crate::guid::JXLWINTHUMB_LIBID;
use crate::guid::JXLWINTHUMB_THUMBNAILPROVIDER_CLSID;

const EXT: &str = ".jxl";

const DESCRIPTION: &str = "JPEG XL File";
const CONTENT_TYPE_KEY: &str = "Content Type";
const CONTENT_TYPE_VALUE: &str = "image/jxl";
const PERCEIVED_TYPE_KEY: &str = "PerceivedType";
const PERCEIVED_TYPE_VALUE: &str = "image";

pub fn register_base(module_path: &str) -> std::io::Result<()> {
    fn register_typelib(module_path: &str) -> std::io::Result<()> {
        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
        let typelib_key = hkcr.open_subkey("TypeLib")?;
        let (key, _) = typelib_key.create_subkey(&format!("{{{:?}}}", JXLWINTHUMB_LIBID))?;
        key.set_value("", &"jxl-winthumb TypeLib")?;

        let (version_key, _) = key.create_subkey("0.1")?;
        version_key.set_value("", &"jxl-winthumb 0.1")?;

        let (first, _) = version_key.create_subkey("0")?;
        let (win64, _) = first.create_subkey("win64")?;
        win64.set_value("", &module_path)?;

        let (flags, _) = version_key.create_subkey("FLAGS")?;
        flags.set_value("", &"0")?;

        Ok(())
    }

    fn register_class(module_path: &str) -> std::io::Result<()> {
        let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
        let clsid_key = hkcr.open_subkey("CLSID")?;
        let (key, _) =
            clsid_key.create_subkey(&format!("{{{:?}}}", JXLWINTHUMB_THUMBNAILPROVIDER_CLSID))?;
        key.set_value("", &"jxl-winthumb")?;

        let (inproc, _) = key.create_subkey("InProcServer32")?;
        inproc.set_value("", &module_path)?;
        inproc.set_value("ThreadingModel", &"Both")?;

        let (prog, _) = key.create_subkey("ProgID")?;
        prog.set_value("", &"jxl-winthumb.ThumbnailProvider.1_0")?;

        let (type_lib, _) = key.create_subkey("TypeLib")?;
        type_lib.set_value("", &format!("{{{:?}}}", JXLWINTHUMB_LIBID))?;

        let (ver_ind, _) = key.create_subkey("VersionIndependentProgID")?;
        ver_ind.set_value("", &"jxl-winthumb.ThumbnailProvider")?;

        Ok(())
    }

    register_typelib(module_path)?;
    register_class(module_path)?;

    Ok(())
}

pub fn unregister_base() -> std::io::Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    let typelib_key = hkcr.open_subkey("TypeLib")?;
    typelib_key.delete_subkey_all(&format!("{{{:?}}}", JXLWINTHUMB_LIBID))?;

    let clsid_key = hkcr.open_subkey("CLSID")?;
    clsid_key.delete_subkey_all(&format!("{{{:?}}}", JXLWINTHUMB_THUMBNAILPROVIDER_CLSID))?;

    Ok(())
}

fn shell_change_notify() {
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, null_mut(), null_mut()) };
}

pub fn register_provider() -> std::io::Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let (key, _) = hkcr.create_subkey(EXT)?;
    key.set_value("", &DESCRIPTION)?;
    key.set_value(CONTENT_TYPE_KEY, &CONTENT_TYPE_VALUE)?;
    key.set_value(PERCEIVED_TYPE_KEY, &PERCEIVED_TYPE_VALUE)?;

    let (shell_ex, _) = key.create_subkey("ShellEx")?;

    let (itp_clsid, _) = shell_ex.create_subkey(&format!(
        "{{{:?}}}",
        crate::bindings::Windows::Win32::UI::Shell::IThumbnailProvider::IID
    ))?;

    itp_clsid.set_value(
        "",
        &format!("{{{:?}}}", JXLWINTHUMB_THUMBNAILPROVIDER_CLSID),
    )?;

    shell_change_notify();

    Ok(())
}

pub fn unregister_provider() -> std::io::Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    if let Ok(key) = hkcr.open_subkey(EXT) {
        if let Ok(shell_ex) = key.open_subkey("ShellEx") {
            if let Ok(itp_clsid) = shell_ex.open_subkey_with_flags(
                &format!(
                    "{{{:?}}}",
                    crate::bindings::Windows::Win32::UI::Shell::IThumbnailProvider::IID
                ),
                KEY_READ | KEY_WRITE,
            ) {
                let rv: Result<String, _> = itp_clsid.get_value("");
                if let Ok(val) = rv {
                    if val == format!("{{{:?}}}", JXLWINTHUMB_THUMBNAILPROVIDER_CLSID) {
                        itp_clsid.delete_value("")?;
                    }
                }
            }
        }
    }

    shell_change_notify();

    Ok(())
}
