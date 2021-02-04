#![crate_type = "dylib"]

use intercom::{prelude::*, raw::HRESULT};
use kagamijxl::Decoder;
use std::{cmp::max, io::BufReader};
use winapi::um::wingdi::{CreateBitmap, DeleteObject};
use winapi::{
    shared::{minwindef::DWORD, windef::HBITMAP, winerror::WINCODEC_ERR_WRONGSTATE},
    um::objidlbase::LPSTREAM,
};

mod registry;
mod winstream;
use winstream::WinStream;

com_library! {
    on_load=on_load,
    on_register=registry::register_provider,
    on_unregister=registry::unregister_provider,
    class ThumbnailProvider
}

/// Called when the DLL is loaded.
///
/// Sets up logging to the Cargo.toml directory for debug purposes.
fn on_load() {
    #[cfg(debug_assertions)]
    {
        // Set up logging to the project directory.
        use log::LevelFilter;
        simple_logging::log_to_file(
            &format!("{}\\debug.log", env!("CARGO_MANIFEST_DIR")),
            LevelFilter::Trace,
        )
        .unwrap();
    }
}

#[com_class(
    // A unique identifier solely for jxl-winthumb
    clsid = "df52deb1-9d07-4520-b606-97c6ecb069a2",
    IInitializeWithStream,
    IThumbnailProvider
)]
#[derive(Default)]
struct ThumbnailProvider {
    stream: Option<LPSTREAM>,
    bitmap: Option<HBITMAP>,
}

impl IInitializeWithStream for ThumbnailProvider {
    fn initialize(&mut self, stream: ComLPSTREAM, _mode: DWORD) -> ComResult<()> {
        unsafe {
            stream.0.as_mut().unwrap().AddRef();
            self.stream = Some(stream.0);
            Ok(())
        }
    }
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

impl IThumbnailProvider for ThumbnailProvider {
    fn get_thumbnail(&mut self, cx: u32) -> ComResult<(ComHBITMAP, WTS_ALPHATYPE)> {
        if self.stream.is_none() {
            return Err(HRESULT::new(WINCODEC_ERR_WRONGSTATE).into());
        }

        let stream = WinStream::new(self.stream.unwrap());
        let reader = BufReader::new(stream);

        let (info, rgba) = {
            let mut decoder = Decoder::new();
            decoder.max_frames = Some(1);

            log::trace!("Decoding started");

            let mut result = decoder.decode_buffer(reader)?;
            let info = result.basic_info;
            let buf = result.frames.remove(0).data;

            log::trace!("Decoding finished");

            let rgba = image::RgbaImage::from_raw(info.xsize, info.ysize, buf)
                .expect("Failed to consume the decoded RGBA buffer");
            (info, rgba)
        };

        let shrink_ratio = max(info.xsize, info.ysize) as f64 / cx as f64;
        let new_size = (
            (info.xsize as f64 / shrink_ratio) as u32,
            (info.ysize as f64 / shrink_ratio) as u32,
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

        Ok((ComHBITMAP(bitmap), 2))
    }
}

impl Drop for ThumbnailProvider {
    fn drop(&mut self) {
        if let Some(stream) = &self.stream {
            unsafe {
                stream.as_mut().unwrap().Release();
            }
        }
        // Delete the bitmap once it's not needed anymore.
        if let Some(bitmap) = self.bitmap {
            unsafe { DeleteObject(bitmap as _) };
        }
    }
}

// New types for deriving Intercom traits.

#[derive(intercom::ForeignType, intercom::ExternType, intercom::ExternOutput)]
#[allow(non_camel_case_types)]
#[repr(transparent)]
struct ComHBITMAP(HBITMAP);

#[derive(
    intercom::ForeignType, intercom::ExternType, intercom::ExternOutput, intercom::ExternInput,
)]
#[allow(non_camel_case_types)]
#[repr(transparent)]
struct ComLPSTREAM(LPSTREAM);

#[allow(non_camel_case_types)]
type WTS_ALPHATYPE = u32;

// COM interface definitions.

#[com_interface(com_iid = "e357fccd-a995-4576-b01f-234630154e96")]
trait IThumbnailProvider {
    fn get_thumbnail(&mut self, cx: u32) -> ComResult<(ComHBITMAP, WTS_ALPHATYPE)>;
}

#[com_interface(com_iid = "b824b49d-22ac-4161-ac8a-9916e8fa3f7f")]
trait IInitializeWithStream {
    fn initialize(&mut self, stream: ComLPSTREAM, mode: DWORD) -> ComResult<()>;
}
