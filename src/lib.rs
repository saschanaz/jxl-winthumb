#![crate_type = "dylib"]

use intercom::prelude::*;
use kagamijxl::decode_memory;
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

com_library! {
    on_load=on_load,
    class ThumbnailProvider
}

/// Called when the DLL is loaded.
///
/// Sets up logging to the Cargo.toml directory for debug purposes.
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

// TODO: do this with encoder option. For now this is for fun
fn fit(vec: &Vec<u8>, size: (u32, u32), max_edge: u32) -> (Vec<u8>, u32, u32) {
    if size <= (max_edge, max_edge) {
        return (vec.clone(), size.0, size.1);
    }

    assert_eq!(vec.len(), (size.0 * size.1 * 4) as usize);

    let shrink_ratio = max(size.0, size.1) as f64 / max_edge as f64;
    let new_size = (
        (size.0 as f64 / shrink_ratio) as u32,
        (size.1 as f64 / shrink_ratio) as u32,
    );

    let new_length = new_size.0 * new_size.1 * 4;
    let mut new = vec![0; new_length as usize];
    for y in 0..new_size.1 {
        for x in 0..new_size.0 {
            let orig_x = (x as f64 * shrink_ratio) as u32;
            let orig_y = (y as f64 * shrink_ratio) as u32;
            let orig_index = orig_y * size.0 * 4 + orig_x * 4;
            let index = y * new_size.0 * 4 + x * 4;
            new[index as usize] = vec[orig_index as usize];
            new[(index + 1) as usize] = vec[(orig_index + 1) as usize];
            new[(index + 2) as usize] = vec[(orig_index + 2) as usize];
            new[(index + 3) as usize] = vec[(orig_index + 3) as usize];
        }
    }

    (new, new_size.0, new_size.1)
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

        log::trace!("data");
        let (info, decoded, _) = decode_memory(&data)?;

        log::trace!("decoded");

        let (mut output, new_xsize, new_ysize) = fit(&decoded, (info.xsize, info.ysize), cx);
        reorder(&mut output);

        // Create a bitmap from the data and return it.
        //
        // We'll store the bitmap handle in the struct so that it can destroy the data when it's not needed anymore.
        let bitmap = unsafe {
            CreateBitmap(
                new_xsize as i32,
                new_ysize as i32,
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
