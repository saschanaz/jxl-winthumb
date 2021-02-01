use std::ptr::null_mut;

use winreg::enums::*;
use winreg::RegKey;

mod bindings {
    ::windows::include_bindings!();
}

use bindings::windows::win32::shell::SHChangeNotify;

const EXT: &str = ".jxl";

const DESCRIPTION: &str = "JPEG XL File";
const CONTENT_TYPE_KEY: &str = "Content Type";
const CONTENT_TYPE_VALUE: &str = "image/jxl";
const PERCEIVED_TYPE_KEY: &str = "PerceivedType";
const PERCEIVED_TYPE_VALUE: &str = "image";

const ITHUMBNAILPROVIDER_CLSID: &str = "{e357fccd-a995-4576-b01f-234630154e96}";
const CLSID: &str = "{DF52DEB1-9D07-4520-B606-97C6ECB069A2}";

fn shell_change_notify() {
    unsafe {
        SHChangeNotify(
            0x08000000, /* SHCNE_ASSOCCHANGED */
            0,          /* SHCNF_IDLIST */
            null_mut(),
            null_mut(),
        )
    };
}

pub fn register_provider() -> Result<(), intercom::raw::HRESULT> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let (key, _) = hkcr.create_subkey(EXT).map_err(|_| intercom::raw::E_FAIL)?;
    key.set_value("", &DESCRIPTION)
        .map_err(|_| intercom::raw::E_FAIL)?;
    key.set_value(CONTENT_TYPE_KEY, &CONTENT_TYPE_VALUE)
        .map_err(|_| intercom::raw::E_FAIL)?;
    key.set_value(PERCEIVED_TYPE_KEY, &PERCEIVED_TYPE_VALUE)
        .map_err(|_| intercom::raw::E_FAIL)?;

    let (shell_ex, _) = key
        .create_subkey("ShellEx")
        .map_err(|_| intercom::raw::E_FAIL)?;

    let (itp_clsid, _) = shell_ex
        .create_subkey(ITHUMBNAILPROVIDER_CLSID)
        .map_err(|_| intercom::raw::E_FAIL)?;

    itp_clsid
        .set_value("", &CLSID)
        .map_err(|_| intercom::raw::E_FAIL)?;

    shell_change_notify();

    Ok(())
}

pub fn unregister_provider() -> Result<(), intercom::raw::HRESULT> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    if let Some(key) = hkcr.open_subkey(EXT).ok() {
        if let Some(shell_ex) = key.open_subkey("ShellEx").ok() {
            if let Some(itp_clsid) = shell_ex
                .open_subkey_with_flags(ITHUMBNAILPROVIDER_CLSID, KEY_READ | KEY_WRITE)
                .ok()
            {
                let rv: Result<String, _> = itp_clsid.get_value("");
                if let Some(val) = rv.ok() {
                    if val == CLSID {
                        itp_clsid
                            .delete_value("")
                            .map_err(|_| intercom::raw::E_FAIL)?;
                    }
                }
            }
        }
    }

    shell_change_notify();

    Ok(())
}
