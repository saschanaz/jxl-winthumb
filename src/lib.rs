#![crate_type = "dylib"]
// XXX: https://github.com/microsoft/windows-rs/issues/1184
#![allow(clippy::forget_copy)]

use kagamijxl::{BasicInfo, DecodeResult, Decoder};
use std::io::BufReader;
use windows::{implement, Guid, Interface};

mod registry;
mod winstream;
use winstream::WinStream;

mod bindings;

use bindings::Windows;
use Windows::{
    Win32::Foundation::{
        E_INVALIDARG, WINCODEC_ERR_BADIMAGE, WINCODEC_ERR_CODECNOTHUMBNAIL,
        WINCODEC_ERR_NOTINITIALIZED, WINCODEC_ERR_PALETTEUNAVAILABLE,
        WINCODEC_ERR_UNSUPPORTEDOPERATION,
    },
    Win32::Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_WICPixelFormat32bppRGBA, IWICBitmapDecoderInfo,
        IWICBitmapFrameDecode, IWICBitmapSource, IWICColorContext, IWICImagingFactory,
        IWICMetadataQueryReader, IWICPalette, WICBitmapDecoderCapabilityCanDecodeAllImages,
        WICBitmapDecoderCapabilityCanDecodeSomeImages, WICDecodeOptions, WICRect,
    },
    Win32::Storage::StructuredStorage::IStream,
    Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
};

mod dll;
mod guid;
use guid::get_guid_from_u128;

#[implement(Windows::Win32::Graphics::Imaging::IWICBitmapDecoder)]
#[derive(Default)]
pub struct JXLWICBitmapDecoder {
    decoded: Option<DecodeResult>,
}

#[allow(non_snake_case)]
#[allow(clippy::missing_safety_doc)]
impl JXLWICBitmapDecoder {
    pub const CLSID: Guid = get_guid_from_u128(0x655896c6_b7d0_4d74_8afb_a02ece3f5e5a);
    pub const CONTAINER_ID: Guid = get_guid_from_u128(0x81e337bc_c1d1_4dee_a17c_402041ba9b5e);

    pub fn QueryCapability(&self, _pistream: &Option<IStream>) -> windows::Result<i32> {
        log::trace!("QueryCapability");
        Ok(WICBitmapDecoderCapabilityCanDecodeSomeImages.0
            | WICBitmapDecoderCapabilityCanDecodeAllImages.0)
    }

    pub fn Initialize(
        &mut self,
        pistream: &Option<IStream>,
        _cacheoptions: WICDecodeOptions,
    ) -> windows::Result<()> {
        log::trace!("JXLWICBitmapDecoder::Initialize");

        let stream = WinStream::from(pistream.to_owned().unwrap());
        let reader = BufReader::new(stream);

        let mut decoder = Decoder::new();

        let result = decoder
            .decode_buffer(reader)
            .map_err(|message| windows::Error::new(WINCODEC_ERR_BADIMAGE, message))?;
        self.decoded = Some(result);

        Ok(())
    }

    pub fn GetContainerFormat(&self) -> windows::Result<Guid> {
        log::trace!("JXLWICBitmapDecoder::GetContainerFormat");
        // Randomly generated
        Ok(Self::CONTAINER_ID)
    }

    pub unsafe fn GetDecoderInfo(&self) -> windows::Result<IWICBitmapDecoderInfo> {
        log::trace!("JXLWICBitmapDecoder::GetDecoderInfo");
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let component_info = factory.CreateComponentInfo(&Self::CLSID)?;
        component_info.cast()
    }

    pub fn CopyPalette(&self, _pipalette: &Option<IWICPalette>) -> windows::Result<()> {
        log::trace!("JXLWICBitmapDecoder::CopyPalette");
        WINCODEC_ERR_PALETTEUNAVAILABLE.ok()
    }

    pub fn GetMetadataQueryReader(&self) -> windows::Result<IWICMetadataQueryReader> {
        log::trace!("JXLWICBitmapDecoder::GetMetadataQueryReader");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }

    pub fn GetPreview(&self) -> windows::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapDecoder::GetPreview");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }

    pub fn GetColorContexts(
        &self,
        _ccount: u32,
        _ppicolorcontexts: *mut Option<IWICColorContext>,
        _pcactualcount: *mut u32,
    ) -> windows::Result<()> {
        log::trace!("JXLWICBitmapDecoder::GetColorContexts");
        WINCODEC_ERR_UNSUPPORTEDOPERATION.ok()
    }

    pub fn GetThumbnail(&self) -> windows::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapDecoder::GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.ok().unwrap_err())
    }

    pub fn GetFrameCount(&self) -> windows::Result<u32> {
        if self.decoded.is_none() {
            return Err(WINCODEC_ERR_NOTINITIALIZED.ok().unwrap_err());
        }

        let frame_count = self.decoded.as_ref().unwrap().frames.len();
        log::trace!("JXLWICBitmapDecoder::GetFrameCount: {}", frame_count);
        Ok(frame_count as u32)
    }

    pub fn GetFrame(&self, index: u32) -> windows::Result<IWICBitmapFrameDecode> {
        if self.decoded.is_none() {
            return Err(WINCODEC_ERR_NOTINITIALIZED.ok().unwrap_err());
        }

        let frame = &self.decoded.as_ref().unwrap().frames[index as usize];
        let basic_info = self.decoded.as_ref().unwrap().basic_info;

        log::trace!("[{}]: {:?}", index, basic_info);
        // A frame decode should not outlive its decoder or the pointer will become invalid
        // Ideally this should use a reference but lifetimes are not supported on COM interfaces.
        let frame_decode = JXLWICBitmapFrameDecode::new(frame.data.as_ptr(), basic_info);
        Ok(frame_decode.into())
    }
}

#[implement(Windows::Win32::Graphics::Imaging::IWICBitmapFrameDecode)]
pub struct JXLWICBitmapFrameDecode {
    /** Can be invalidated if the decoder gets destroyed */
    data_ptr: *const u8,
    basic_info: BasicInfo,
}

#[allow(non_snake_case)]
#[allow(clippy::missing_safety_doc)]
impl JXLWICBitmapFrameDecode {
    pub fn new(data: *const u8, basic_info: BasicInfo) -> Self {
        Self {
            data_ptr: data,
            basic_info,
        }
    }

    pub unsafe fn GetSize(&self, puiwidth: *mut u32, puiheight: *mut u32) -> windows::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::GetSize");
        *puiwidth = self.basic_info.xsize;
        *puiheight = self.basic_info.ysize;
        Ok(())
    }

    pub fn GetPixelFormat(&self) -> windows::Result<Guid> {
        log::trace!("JXLWICBitmapFrameDecode::GetPixelFormat");
        // TODO: Support HDR
        Ok(GUID_WICPixelFormat32bppRGBA)
    }

    pub unsafe fn GetResolution(&self, pdpix: *mut f64, pdpiy: *mut f64) -> windows::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::GetResolution");
        // TODO: Does JXL have resolution info?
        *pdpix = 96f64;
        *pdpiy = 96f64;
        Ok(())
    }

    pub fn CopyPalette(&self, _pipalette: &Option<IWICPalette>) -> windows::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::CopyPalette");
        WINCODEC_ERR_PALETTEUNAVAILABLE.ok()
    }

    pub unsafe fn CopyPixels(
        &self,
        prc: *const WICRect,
        _cbstride: u32,
        _cbbuffersize: u32,
        pbbuffer: *mut u8,
    ) -> windows::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::CopyPixels");

        if prc.is_null() {
            return Err(E_INVALIDARG.ok().unwrap_err());
        }

        let prc = prc.as_ref().unwrap();

        for y in prc.Y..(prc.Y + prc.Height) {
            let src_offset = self.basic_info.xsize as i32 * 4 * y;
            let dst_offset = prc.Width * 4 * (prc.Y - y);
            std::ptr::copy_nonoverlapping(
                self.data_ptr.offset((src_offset + prc.X) as isize),
                pbbuffer.offset(dst_offset as isize),
                (prc.Width as usize) * 4,
            );
        }

        Ok(())
    }

    pub fn GetMetadataQueryReader(&self) -> windows::Result<IWICMetadataQueryReader> {
        log::trace!("JXLWICBitmapFrameDecode::GetMetadataQueryReader");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }

    pub unsafe fn GetColorContexts(
        &self,
        _ccount: u32,
        _ppicolorcontexts: *mut Option<IWICColorContext>,
        pcactualcount: *mut u32,
    ) -> windows::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::GetColorContexts");
        *pcactualcount = 0;
        Ok(())
    }

    pub fn GetThumbnail(&self) -> windows::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapFrameDecode::GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.ok().unwrap_err())
    }
}
