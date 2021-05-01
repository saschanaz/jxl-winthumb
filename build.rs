fn main() {
    windows::build!(
      Windows::Win32::Shell::{SHChangeNotify, WTS_ALPHATYPE},
      Windows::Win32::Gdi::{CreateBitmap, DeleteObject, HBITMAP},
      Windows::Win32::StructuredStorage::{ISequentialStream, IStream},
      Windows::Win32::SystemServices::WINCODEC_ERR_WRONGSTATE,
    );
}
