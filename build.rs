fn main() {
    windows::build!(
      Windows::Win32::UI::Shell::{SHChangeNotify, WTS_ALPHATYPE, WTSAT_ARGB, SHCNE_ASSOCCHANGED, SHCNF_IDLIST},
      Windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject, HBITMAP},
      Windows::Win32::Storage::StructuredStorage::{ISequentialStream, IStream},
      Windows::Win32::System::SystemServices::WINCODEC_ERR_WRONGSTATE,
    );
}
