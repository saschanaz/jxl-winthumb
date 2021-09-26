use std::ptr::null_mut;

use windows::Interface;
use winreg::enums::*;
use winreg::RegKey;
use winreg::RegValue;

use crate::bindings::Windows::Win32::UI::Shell::{
    SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
};
use crate::guid::guid_to_string;
use crate::guid::JXLWINTHUMB_THUMBNAILPROVIDER_CLSID;
use crate::guid::JXLWINTHUMB_VENDOR_CLSID;
use crate::wic::JXLWICBitmapDecoder;

const EXT: &str = ".jxl";

const DESCRIPTION: &str = "JPEG XL File";
const CONTENT_TYPE_KEY: &str = "Content Type";
const CONTENT_TYPE_VALUE: &str = "image/jxl";
const PERCEIVED_TYPE_KEY: &str = "PerceivedType";
const PERCEIVED_TYPE_VALUE: &str = "image";

fn register_clsid_base(module_path: &str, clsid: &windows::Guid) -> std::io::Result<RegKey> {
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

pub fn register_clsid(module_path: &str) -> std::io::Result<()> {
    register_clsid_base(module_path, &JXLWINTHUMB_THUMBNAILPROVIDER_CLSID)?;

    let wic_decoder_key = register_clsid_base(module_path, &JXLWICBitmapDecoder::CLSID)?;
    // Required entries
    // https://docs.microsoft.com/en-us/windows/win32/wic/-wic-generalregentries
    wic_decoder_key.set_value("FriendlyName", &"jxl-winthumb WIC Decoder")?;
    wic_decoder_key.set_value("VendorGUID", &guid_to_string(&JXLWINTHUMB_VENDOR_CLSID))?;
    wic_decoder_key.set_value("MimeTypes", &CONTENT_TYPE_VALUE)?;
    wic_decoder_key.set_value("FileExtensions", &EXT)?;
    wic_decoder_key.set_value(
        "ContainerFormat",
        &guid_to_string(&JXLWICBitmapDecoder::CONTAINER_ID),
    )?;

    let (formats, _) = wic_decoder_key.create_subkey("Formats")?;
    formats.create_subkey(guid_to_string(
        &crate::bindings::Windows::Win32::Graphics::Imaging::GUID_WICPixelFormat32bppRGBA,
    ))?;

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

pub fn unregister_clsid() -> std::io::Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    let clsid_key = hkcr.open_subkey("CLSID")?;
    clsid_key.delete_subkey_all(&guid_to_string(&JXLWINTHUMB_THUMBNAILPROVIDER_CLSID))?;
    clsid_key.delete_subkey_all(&guid_to_string(&JXLWICBitmapDecoder::CLSID))?;

    let instances_key = clsid_key
        .open_subkey("{7ED96837-96F0-4812-B211-F13C24117ED3}")?
        .open_subkey("Instance")?;

    instances_key.delete_subkey_all(&guid_to_string(&JXLWICBitmapDecoder::CLSID))?;

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

    let (system_assoc, _) = hkcr.create_subkey("SystemFileAssociations")?;
    let (system_key, _) = system_assoc.create_subkey(EXT)?;
    let (system_open_with, _) = system_key.create_subkey("OpenWithList")?;
    system_open_with.create_subkey("PhotoViewer.dll")?;

    let (shell_ex, _) = key.create_subkey("ShellEx")?;

    let (itp_clsid, _) = shell_ex.create_subkey(&guid_to_string(
        &crate::bindings::Windows::Win32::UI::Shell::IThumbnailProvider::IID,
    ))?;

    // itp_clsid.set_value("", &guid_to_string(&JXLWINTHUMB_THUMBNAILPROVIDER_CLSID))?;

    // https://docs.microsoft.com/en-us/windows/win32/wic/-wic-integrationregentries#integration-with-the-windows-thumbnail-cache
    itp_clsid.set_value("", &"{C7657C4A-9F68-40fa-A4DF-96BC08EB3551}")?;

    let (open_with, _) = key.create_subkey("OpenWithList")?;
    open_with.create_subkey("PhotoViewer.dll")?;

    // let (property_handler, _) = key.create_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\PropertySystem\\PropertyHandlers\\.jxl")?;
    // // TODO: wrong CLSID for now, just for test
    // property_handler.set_value("", &guid_to_string(&JXLWICBitmapDecoder::CLSID))?;

    shell_change_notify();

    Ok(())
}

pub fn unregister_provider() -> std::io::Result<()> {
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    if let Ok(key) = hkcr.open_subkey(EXT) {
        if let Ok(shell_ex) = key.open_subkey("ShellEx") {
            if let Ok(itp_clsid) = shell_ex.open_subkey_with_flags(
                &guid_to_string(
                    &crate::bindings::Windows::Win32::UI::Shell::IThumbnailProvider::IID,
                ),
                KEY_READ | KEY_WRITE,
            ) {
                let rv: Result<String, _> = itp_clsid.get_value("");
                if let Ok(val) = rv {
                    if val == guid_to_string(&JXLWINTHUMB_THUMBNAILPROVIDER_CLSID) {
                        itp_clsid.delete_value("")?;
                    }
                }
            }
        }
    }

    shell_change_notify();

    Ok(())
}
