use winreg::RegKey;
use winreg::enums::*;

fn open_kindmap_key() -> std::io::Result<RegKey> {
    let hkcr = RegKey::predef(HKEY_LOCAL_MACHINE);
    hkcr.open_subkey_with_flags(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\KindMap",
        KEY_WRITE,
    )
}

pub fn register_explorer_kind() -> std::io::Result<()> {
    open_kindmap_key()?.set_value(".jxl", &"picture")
}

pub fn unregister_explorer_kind() -> std::io::Result<()> {
    open_kindmap_key()?.delete_value(".jxl")
}
