use windows::core::GUID;

pub const JXLWINTHUMB_VENDOR_CLSID: GUID = GUID::from_u128(0x448d5eb7_6555_476b_a840_034cca9afe6e);

pub fn guid_to_string(guid: &GUID) -> String {
    format!("{{{:?}}}", guid)
}
