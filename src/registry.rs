use windows::core::Interface;
use winreg::enums::*;
use winreg::types::ToRegValue;
use winreg::RegKey;
use winreg::RegValue;

use crate::guid::{guid_to_string, JXLWINTHUMB_VENDOR_CLSID};
use crate::JXLWICBitmapDecoder;

mod kindmap;
mod property_handler;

const EXT: &str = ".jxl";

const PROGID: &str = "jxlwinthumbfile";
const CONTENT_TYPE_KEY: &str = "Content Type";
const CONTENT_TYPE_VALUE: &str = "image/jxl";
const PERCEIVED_TYPE_KEY: &str = "PerceivedType";
const PERCEIVED_TYPE_VALUE: &str = "image";

fn register_clsid_base(module_path: &str, clsid: &windows::core::GUID) -> std::io::Result<RegKey> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let clsid_key = hkcr.open_subkey("CLSID")?;
    let (key, _) = clsid_key.create_subkey(&guid_to_string(clsid))?;
    key.set_value("", &"jxl-winthumb")?;

    let (inproc, _) = key.create_subkey("InProcServer32")?;
    inproc.set_value("", &module_path)?;
    inproc.set_value("ThreadingModel", &"Both")?;

    Ok(key)
}

fn open_clsid(key: &str) -> std::io::Result<RegKey> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let clsid_key = hkcr.open_subkey("CLSID")?;
    clsid_key.open_subkey(key)
}

fn set_pattern(key: &RegKey, pattern: Vec<u8>) -> std::io::Result<()> {
    let len = pattern.len();

    key.set_value("Position", &0u32)?;
    key.set_value("Length", &(len as u32))?;
    key.set_raw_value(
        "Pattern",
        &RegValue {
            vtype: REG_BINARY,
            bytes: pattern,
        },
    )?;
    key.set_raw_value(
        "Mask",
        &RegValue {
            vtype: REG_BINARY,
            bytes: vec![0xff; len],
        },
    )?;

    Ok(())
}

fn register_clsid(module_path: &str) -> std::io::Result<()> {
    let wic_decoder_key = register_clsid_base(module_path, &JXLWICBitmapDecoder::CLSID)?;
    // General required entries
    // https://docs.microsoft.com/en-us/windows/win32/wic/-wic-generalregentries
    wic_decoder_key.set_value("FriendlyName", &"jxl-winthumb WIC Decoder")?;
    wic_decoder_key.set_value("VendorGUID", &guid_to_string(&JXLWINTHUMB_VENDOR_CLSID))?;
    wic_decoder_key.set_value("MimeTypes", &CONTENT_TYPE_VALUE)?;
    wic_decoder_key.set_value("FileExtensions", &EXT)?;

    let (formats, _) = wic_decoder_key.create_subkey("Formats")?;
    formats.create_subkey(guid_to_string(
        &windows::Win32::Graphics::Imaging::GUID_WICPixelFormat32bppRGBA,
    ))?;

    // Decoder specific required entries
    // https://docs.microsoft.com/en-us/windows/win32/wic/-wic-decoderregentries
    let (patterns, _) = wic_decoder_key.create_subkey("Patterns")?;
    let (bytestream_pattern, _) = patterns.create_subkey("0")?;
    set_pattern(&bytestream_pattern, vec![0xff, 0x0a])?;
    let (container_pattern, _) = patterns.create_subkey("1")?;
    set_pattern(
        &container_pattern,
        vec![
            0x00, 0x00, 0x00, 0x0c, 0x4a, 0x58, 0x4c, 0x20, 0x0d, 0x0a, 0x87, 0x0a,
        ],
    )?;

    let instances_key =
        open_clsid("{7ED96837-96F0-4812-B211-F13C24117ED3}")?.open_subkey("Instance")?;
    let (instance_key, _) =
        instances_key.create_subkey(guid_to_string(&JXLWICBitmapDecoder::CLSID))?;
    instance_key.set_value("CLSID", &guid_to_string(&JXLWICBitmapDecoder::CLSID))?;
    instance_key.set_value("FriendlyName", &"jxl-winthumb WIC Decoder")?;

    Ok(())
}

fn unregister_clsid() {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    hkcr.delete_subkey_all(format!(
        "CLSID\\{}",
        &guid_to_string(&JXLWICBitmapDecoder::CLSID)
    ))
    .ok();

    hkcr.delete_subkey_all(format!(
        "CLSID\\{{7ED96837-96F0-4812-B211-F13C24117ED3}}\\Instance\\{}",
        &guid_to_string(&JXLWICBitmapDecoder::CLSID)
    ))
    .ok();
}

fn create_expand_sz(value: &str) -> RegValue {
    RegValue {
        vtype: winreg::enums::REG_EXPAND_SZ,
        bytes: value.to_reg_value().bytes,
    }
}

fn register_property_list(system_ext_key: &RegKey) -> std::io::Result<()> {
    // https://docs.microsoft.com/en-us/windows/win32/properties/building-property-handlers-property-lists
    // The example uses HKCR\.ext but somehow the system actually uses HKCR\SystemFileAssociations\.ext instead.

    // Copied from other system file associations and trimmed down.
    system_ext_key.set_value("FullDetails", &"prop:System.PropGroup.Image;System.Image.Dimensions;System.Image.HorizontalSize;System.Image.VerticalSize;System.PropGroup.FileSystem;System.ItemNameDisplay;System.ItemType;System.ItemFolderPathDisplay;System.DateCreated;System.DateModified;System.Size;System.FileAttributes;System.OfflineAvailability;System.OfflineStatus;System.SharedWith;System.FileOwner;System.ComputerName")?;
    system_ext_key.set_value("PreviewDetails", &"prop:*System.Image.Dimensions;*System.Size;*System.OfflineAvailability;*System.OfflineStatus;*System.DateCreated;*System.DateModified;*System.DateAccessed;*System.SharedWith")?;
    Ok(())
}

fn register_provider() -> std::io::Result<()> {
    // Integration with the Windows Photo Gallery
    // https://docs.microsoft.com/en-us/windows/win32/wic/-wic-integrationregentries#integration-with-the-windows-photo-gallery
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let (ext_key, _) = hkcr.create_subkey(EXT)?;
    ext_key.set_value("", &PROGID)?;
    ext_key.set_value(CONTENT_TYPE_KEY, &CONTENT_TYPE_VALUE)?;
    ext_key.set_value(PERCEIVED_TYPE_KEY, &PERCEIVED_TYPE_VALUE)?;

    ext_key.create_subkey(format!("OpenWithProgids\\{}", PROGID))?;

    let (system_ext_key, _) = hkcr.create_subkey(format!("SystemFileAssociations\\{}", EXT))?;
    system_ext_key
        .create_subkey("ShellEx\\ContextMenuHandlers\\ShellImagePreview")?
        .0
        .set_value("", &"{FFE2A43C-56B9-4bf5-9A79-CC6D4285608A}")?;
    register_property_list(&system_ext_key)?;

    let (progid_key, _) = hkcr.create_subkey(PROGID)?;
    progid_key.set_value("", &"JXL File")?;
    let (progid_shell_key, _) = progid_key.create_subkey("shell")?;
    let (open_key, _) = progid_shell_key.create_subkey("open")?;
    open_key.set_raw_value(
        "MuiVerb",
        &create_expand_sz("@%PROGRAMFILES%\\Windows Photo Viewer\\photoviewer.dll,-3043"),
    )?;
    open_key.create_subkey("command")?.0.set_raw_value("", &create_expand_sz("%SystemRoot%\\System32\\rundll32.exe \"%ProgramFiles%\\Windows Photo Viewer\\PhotoViewer.dll\", ImageView_Fullscreen %1"))?;
    open_key
        .create_subkey("DropTarget")?
        .0
        .set_value("", &"{FFE2A43C-56B9-4bf5-9A79-CC6D4285608A}")?;
    progid_shell_key.create_subkey("printto\\command")?.0.set_raw_value("name", &create_expand_sz("%SystemRoot%\\System32\\rundll32.exe \"%SystemRoot%\\System32\\shimgvw.dll\", ImageView_PrintTo /pt \"%1\" \"%2\" \"%3\" \"%4\""))?;

    // Integration with the Windows Thumbnail Cache
    // https://docs.microsoft.com/en-us/windows/win32/wic/-wic-integrationregentries#integration-with-the-windows-thumbnail-cache
    let (system_shell_ex, _) = system_ext_key.create_subkey("ShellEx")?;
    system_shell_ex
        .create_subkey(&guid_to_string(
            &windows::Win32::UI::Shell::IThumbnailProvider::IID,
        ))?
        .0
        .set_value("", &"{C7657C4A-9F68-40fa-A4DF-96BC08EB3551}")?;

    Ok(())
}

fn delete_default_if_same(subkey_path: &str, value: &str) -> std::io::Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    if let Ok(subkey) = hkcr.open_subkey_with_flags(subkey_path, KEY_READ | KEY_WRITE) {
        let rv: Result<String, _> = subkey.get_value("");
        if let Ok(val) = rv {
            if val == value {
                subkey.delete_value("")?;
            }
        }
    }
    Ok(())
}

fn unregister_provider() -> std::io::Result<()> {
    delete_default_if_same(
        &format!(
            "SystemFileAssociations\\{}\\ShellEx\\{{{:?}}}",
            EXT,
            windows::Win32::UI::Shell::IThumbnailProvider::IID
        ),
        "{C7657C4A-9F68-40fa-A4DF-96BC08EB3551}",
    )?;

    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    hkcr.delete_subkey_all(format!("{}\\OpenWithProgids\\{}", EXT, PROGID))
        .ok();

    delete_default_if_same(
        &format!(
            "{}\\ShellEx\\{{{:?}}}",
            EXT,
            windows::Win32::UI::Shell::IThumbnailProvider::IID
        ),
        "{C7657C4A-9F68-40fa-A4DF-96BC08EB3551}",
    )?;
    hkcr.delete_subkey_all(format!("{}\\OpenWithList", EXT))
        .ok();
    hkcr.delete_subkey_all(format!("SystemFileAssociations\\{}\\OpenWithList", EXT))
        .ok();

    hkcr.delete_subkey_all(PROGID).ok();

    Ok(())
}

pub fn register(module_path: &str) -> std::io::Result<()> {
    register_clsid(module_path)?;
    register_provider()?;
    kindmap::register_explorer_kind()?;
    property_handler::register_property_handler(module_path)?;
    Ok(())
}

pub fn unregister() -> std::io::Result<()> {
    unregister_clsid();
    unregister_provider()?;
    kindmap::unregister_explorer_kind().ok();
    property_handler::unregister_property_handler().ok();
    Ok(())
}
