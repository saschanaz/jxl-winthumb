fn main() {
    windows::build!(
      windows::win32::shell::{SHChangeNotify, WTS_ALPHATYPE},
      windows::win32::gdi::{CreateBitmap, DeleteObject, HBITMAP},
      windows::win32::structured_storage::{ISequentialStream, IStream},
      windows::win32::system_services::WINCODEC_ERR_WRONGSTATE,
    );
}
