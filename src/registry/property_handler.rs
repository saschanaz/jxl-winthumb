use winreg::enums::*;
use winreg::RegKey;

use crate::guid::guid_to_string;
use crate::properties::JXLPropertyStore;

use super::{register_clsid_base, EXT};

const PROPERTY_HANDLERS_KEY: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\PropertySystem\\PropertyHandlers";

pub fn register_property_handler(module_path: &str) -> std::io::Result<()> {
    // https://docs.microsoft.com/en-us/windows/win32/properties/prophand-reg-dist

    // No ManualSafeSave needed since it's currently read-only
    register_clsid_base(module_path, &JXLPropertyStore::CLSID)?;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let handlers_key = hklm.open_subkey(PROPERTY_HANDLERS_KEY)?;
    let (handler_key, _) = handlers_key.create_subkey_with_flags(EXT, KEY_WRITE)?;

    handler_key.set_value("", &guid_to_string(&JXLPropertyStore::CLSID))?;

    Ok(())
}

pub fn unregister_property_handler() -> std::io::Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    let clsid_key = hkcr.open_subkey("CLSID")?;
    clsid_key
        .delete_subkey_all(guid_to_string(&JXLPropertyStore::CLSID))
        .ok();

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let handlers_key = hklm.open_subkey(PROPERTY_HANDLERS_KEY)?;
    handlers_key.delete_subkey_all(EXT).ok();

    Ok(())
}
