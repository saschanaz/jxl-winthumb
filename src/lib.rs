#![crate_type = "dylib"]

use kagamijxl::Decoder;
use std::{cmp::max, io::BufReader};
use windows::implement;

mod registry;
mod winstream;
use winstream::WinStream;

mod bindings;

use bindings::Windows;
use Windows::{
    Win32::Foundation::{WINCODEC_ERR_BADIMAGE, WINCODEC_ERR_WRONGSTATE},
    Win32::Graphics::Gdi::{CreateBitmap, DeleteObject, HBITMAP},
    Win32::Storage::StructuredStorage::IStream,
    Win32::UI::Shell::{WTSAT_ARGB, WTS_ALPHATYPE},
};

mod dll;
mod guid;
mod wic;

#[implement(
    Windows::Win32::System::PropertiesSystem::IInitializeWithStream,
    Windows::Win32::UI::Shell::IThumbnailProvider
)]
#[derive(Default)]
struct ThumbnailProvider {
    stream: Option<WinStream>,
    bitmap: Option<HBITMAP>,
}

// TODO: Use encoder channel order option when available. Not yet as of 0.3.0
fn reorder(vec: &mut Vec<u8>) {
    assert_eq!(vec.len() % 4, 0);
    for i in 0..vec.len() / 4 {
        // Windows expects BGRA (ARGB in reverse order) while JXL emits RGBA
        let r = vec[i * 4];
        let b = vec[i * 4 + 2];
        vec[i * 4] = b;
        vec[i * 4 + 2] = r;
    }
}

#[allow(non_snake_case)]
impl ThumbnailProvider {
    // IInitializeWithStream::Initialize
    fn Initialize(&mut self, stream: &Option<IStream>, _grfmode: u32) -> windows::Result<()> {
        if stream.is_none() {
            return Err(windows::Error::new(
                WINCODEC_ERR_WRONGSTATE,
                "Expected an IStream object but got none",
            ));
        }
        self.stream = Some(WinStream::from(stream.to_owned().unwrap()));
        Ok(())
    }

    // IThumbnailProvider::GetThumbnail
    fn GetThumbnail(
        &mut self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdwalpha: *mut WTS_ALPHATYPE,
    ) -> windows::Result<()> {
        if self.stream.is_none() {
            return Err(windows::Error::new(
                WINCODEC_ERR_WRONGSTATE,
                "Haven't got the stream yet",
            ));
        }

        let stream = self.stream.take().unwrap();
        let reader = BufReader::new(stream);

        let (info, rgba) = {
            let mut decoder = Decoder::new();
            decoder.max_frames = Some(1);

            log::trace!("Decoding started");

            let mut result = decoder
                .decode_buffer(reader)
                .map_err(|message| windows::Error::new(WINCODEC_ERR_BADIMAGE, message))?;
            let info = result.basic_info;
            let buf = result.frames.remove(0).data;

            log::trace!("Decoding finished");

            let rgba = image::RgbaImage::from_raw(info.xsize, info.ysize, buf)
                .expect("Failed to consume the decoded RGBA buffer");
            (info, rgba)
        };

        let mut shrink_ratio = max(info.xsize, info.ysize) as f64 / cx as f64;
        if shrink_ratio < 1.0 {
            shrink_ratio = 1.0; // cmp::min does not support floats
        }
        let new_size = (
            (info.xsize as f64 / shrink_ratio).round() as u32,
            (info.ysize as f64 / shrink_ratio).round() as u32,
        );

        log::trace!("Resizing/reordering started");

        let resized =
            image::imageops::resize(&rgba, new_size.0, new_size.1, image::imageops::Triangle);
        let mut output = resized.to_vec();
        reorder(&mut output);

        log::trace!("Resizing/reordering finished");

        // Create a bitmap from the data and return it.
        //
        // We'll store the bitmap handle in the struct so that it can destroy the data when it's not needed anymore.
        let bitmap = unsafe {
            CreateBitmap(
                new_size.0 as i32,
                new_size.1 as i32,
                1,
                32,
                output.as_ptr() as *const _,
            )
        };
        self.bitmap = Some(bitmap);

        unsafe {
            *phbmp = bitmap;
            *pdwalpha = WTSAT_ARGB;
        }

        Ok(())
    }
}

impl Drop for ThumbnailProvider {
    fn drop(&mut self) {
        // Delete the bitmap once it's not needed anymore.
        if let Some(bitmap) = self.bitmap {
            unsafe { DeleteObject(bitmap) };
        }
    }
}
