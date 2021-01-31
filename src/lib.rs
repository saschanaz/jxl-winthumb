#![crate_type = "dylib"]

use intercom::prelude::*;
use kagamijxl::Decoder;
use std::{cmp::max, ffi::c_void};
use winapi::um::{
    objidlbase::STATSTG,
    wingdi::{CreateBitmap, DeleteObject},
};
use winapi::{
    shared::{
        minwindef::DWORD, windef::HBITMAP, winerror::WINCODEC_ERR_WRONGSTATE,
        wtypes::STATFLAG_NONAME,
    },
    um::objidlbase::LPSTREAM,
};

#[cfg(not(debug_assertions))]
com_library! {
    class ThumbnailProvider
}

#[cfg(debug_assertions)]
com_library! {
    on_load=on_load,
    class ThumbnailProvider
}

/// Called when the DLL is loaded.
///
/// Sets up logging to the Cargo.toml directory for debug purposes.
#[cfg(debug_assertions)]
fn on_load() {
    // Set up logging to the project directory.
    use log::LevelFilter;
    simple_logging::log_to_file(
        &format!("{}\\debug.log", env!("CARGO_MANIFEST_DIR")),
        LevelFilter::Trace,
    )
    .unwrap();
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

// TODO: Use encoder channel order option when available. Not yet as of 0.2.0
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
            return Err(intercom::error::raw::HRESULT::new(WINCODEC_ERR_WRONGSTATE).into());
        }

        let stream = unsafe { self.stream.unwrap().as_mut().unwrap() };

        let data = unsafe {
            let mut stat = std::mem::zeroed();

            stream.Stat(&mut stat, STATFLAG_NONAME);

            let stream_size = *stat.cbSize.QuadPart() as u32;

            let mut buffer: Vec<u8> = Vec::new();
            buffer.resize(stream_size as usize, 0);
            let mut bytes_read = 0u32;
            while bytes_read < stream_size {
                let offset = buffer.as_mut_ptr().offset(bytes_read as isize);
                stream.Read(offset as *mut c_void, stream_size, &mut bytes_read);
            }
            buffer
        };

        let (info, rgba) = {
            let mut decoder = Decoder::new();
            decoder.max_frames = Some(1);

            let mut result = decoder.decode(&data)?;
            let info = result.basic_info;
            let buf = result.frames.remove(0).data;

            let rgba = image::RgbaImage::from_raw(info.xsize, info.ysize, buf)
                .expect("Failed to consume the decoded RGBA buffer");
            (info, rgba)
        };

        let shrink_ratio = max(info.xsize, info.ysize) as f64 / cx as f64;
        let new_size = (
            (info.xsize as f64 / shrink_ratio) as u32,
            (info.ysize as f64 / shrink_ratio) as u32,
        );

        let resized =
            image::imageops::resize(&rgba, new_size.0, new_size.1, image::imageops::Triangle);
        let mut output = resized.to_vec();
        reorder(&mut output);

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
struct ComSTATSTG(STATSTG);

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
