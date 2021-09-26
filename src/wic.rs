use std::io::BufReader;

use kagamijxl::{BasicInfo, DecodeResult, Decoder};
use windows::{implement, Guid, Interface};

use crate::{bindings::Windows, guid::get_guid_from_u128, winstream::WinStream};
use Windows::{
    Win32::Foundation::{
        WINCODEC_ERR_BADIMAGE, WINCODEC_ERR_CODECNOTHUMBNAIL, WINCODEC_ERR_NOTINITIALIZED,
        WINCODEC_ERR_PALETTEUNAVAILABLE, WINCODEC_ERR_UNSUPPORTEDOPERATION,
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

#[implement(Windows::Win32::Graphics::Imaging::IWICBitmapDecoder)]
#[derive(Default)]
pub struct JXLWICBitmapDecoder {
    decoded: Option<DecodeResult>,
}

#[allow(non_snake_case)]
impl JXLWICBitmapDecoder {
    pub const CLSID: Guid = get_guid_from_u128(0x655896c6_b7d0_4d74_8afb_a02ece3f5e5a);
    pub const CONTAINER_ID: Guid = get_guid_from_u128(0x81e337bc_c1d1_4dee_a17c_402041ba9b5e);

    pub unsafe fn QueryCapability(&self, _pistream: &Option<IStream>) -> windows::Result<i32> {
        log::trace!("QueryCapability");
        Ok(WICBitmapDecoderCapabilityCanDecodeSomeImages.0
            | WICBitmapDecoderCapabilityCanDecodeAllImages.0)
    }
    pub unsafe fn Initialize(
        &mut self,
        pistream: &Option<IStream>,
        _cacheoptions: WICDecodeOptions,
    ) -> windows::Result<()> {
        let stream = WinStream::from(pistream.to_owned().unwrap());
        let reader = BufReader::new(stream);

        let mut decoder = Decoder::new();
        decoder.max_frames = Some(1);

        log::trace!("Decoding started");

        let result = decoder
            .decode_buffer(reader)
            .map_err(|message| windows::Error::new(WINCODEC_ERR_BADIMAGE, message))?;
        self.decoded = Some(result);

        Ok(())
    }
    pub unsafe fn GetContainerFormat(&self) -> windows::Result<Guid> {
        log::trace!("GetContainerFormat");
        // Randomly generated
        Ok(Self::CONTAINER_ID)
    }
    pub unsafe fn GetDecoderInfo(&self) -> windows::Result<IWICBitmapDecoderInfo> {
        log::trace!("GetDecoderInfo");
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let component_info = factory.CreateComponentInfo(&Self::CLSID)?;
        component_info.cast()
    }
    pub unsafe fn CopyPalette(&self, _pipalette: &Option<IWICPalette>) -> windows::Result<()> {
        log::trace!("CopyPalette");
        WINCODEC_ERR_PALETTEUNAVAILABLE.ok()
    }
    pub unsafe fn GetMetadataQueryReader(&self) -> windows::Result<IWICMetadataQueryReader> {
        log::trace!("GetMetadataQueryReader");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }
    pub unsafe fn GetPreview(&self) -> windows::Result<IWICBitmapSource> {
        log::trace!("GetPreview");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }
    pub unsafe fn GetColorContexts(
        &self,
        _ccount: u32,
        _ppicolorcontexts: *mut Option<IWICColorContext>,
        _pcactualcount: *mut u32,
    ) -> windows::Result<()> {
        log::trace!("GetColorContexts");
        WINCODEC_ERR_UNSUPPORTEDOPERATION.ok()
    }
    pub unsafe fn GetThumbnail(&self) -> windows::Result<IWICBitmapSource> {
        log::trace!("GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.ok().unwrap_err())
    }
    pub unsafe fn GetFrameCount(&self) -> windows::Result<u32> {
        if self.decoded.is_none() {
            return Err(WINCODEC_ERR_NOTINITIALIZED.ok().unwrap_err());
        }

        let frame_count = self.decoded.as_ref().unwrap().frames.len();
        log::trace!("Frame count: {}", frame_count);
        Ok(frame_count as u32)
    }
    pub unsafe fn GetFrame(&self, index: u32) -> windows::Result<IWICBitmapFrameDecode> {
        if self.decoded.is_none() {
            return Err(WINCODEC_ERR_NOTINITIALIZED.ok().unwrap_err());
        }

        let frame = &self.decoded.as_ref().unwrap().frames[index as usize];
        let basic_info = self.decoded.as_ref().unwrap().basic_info;

        log::trace!("[{}]: {:?}", index, basic_info);
        // TODO: this is copying the whole frame data, could it be prevented?
        let frame_decode = JXLWICBitmapFrameDecode::new(frame.data.clone(), basic_info);
        frame_decode.cast()

        // Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }
}

#[implement(Windows::Win32::Graphics::Imaging::IWICBitmapFrameDecode)]
#[derive(Default)]
pub struct JXLWICBitmapFrameDecode {
    data: Vec<u8>,
    basic_info: BasicInfo,
}

#[allow(non_snake_case)]
impl JXLWICBitmapFrameDecode {
    pub fn new(data: Vec<u8>, basic_info: BasicInfo) -> Self {
        Self { data, basic_info }
    }

    pub unsafe fn GetSize(&self, puiwidth: *mut u32, puiheight: *mut u32) -> windows::Result<()> {
        log::trace!("GetSize");
        *puiwidth = self.basic_info.xsize;
        *puiheight = self.basic_info.ysize;
        Ok(())
    }
    pub unsafe fn GetPixelFormat(&self) -> windows::Result<Guid> {
        log::trace!("GetPixelFormat");
        // TODO: Support HDR
        Ok(GUID_WICPixelFormat32bppRGBA)
    }
    pub unsafe fn GetResolution(&self, pdpix: *mut f64, pdpiy: *mut f64) -> windows::Result<()> {
        log::trace!("GetResolution");
        // TODO: Does JXL have resolution info?
        *pdpix = 96f64;
        *pdpiy = 96f64;
        Ok(())
    }
    pub unsafe fn CopyPalette<'a>(&self, _pipalette: &Option<IWICPalette>) -> windows::Result<()> {
        log::trace!("CopyPalette");
        WINCODEC_ERR_PALETTEUNAVAILABLE.ok()
    }
    pub unsafe fn CopyPixels(
        &self,
        _prc: *const WICRect,
        _cbstride: u32,
        _cbbuffersize: u32,
        pbbuffer: *mut u8,
    ) -> windows::Result<()> {
        log::trace!("CopyPixels");
        std::ptr::copy_nonoverlapping(self.data.as_ptr(), pbbuffer, self.data.len());
        Ok(())
    }
    pub unsafe fn GetMetadataQueryReader(&self) -> windows::Result<IWICMetadataQueryReader> {
        log::trace!("MetadataQueryReader");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }
    pub unsafe fn GetColorContexts(
        &self,
        _ccount: u32,
        _ppicolorcontexts: *mut Option<IWICColorContext>,
        pcactualcount: *mut u32,
    ) -> windows::Result<()> {
        *pcactualcount = 0;
        Ok(())
    }
    pub unsafe fn GetThumbnail(&self) -> windows::Result<IWICBitmapSource> {
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.ok().unwrap_err())
    }
}
