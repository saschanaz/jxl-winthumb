#![crate_type = "dylib"]
// https://github.com/microsoft/windows-rs/issues/1506
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use kagamijxl::{DecodeProgress, Decoder};
use std::{cell::RefCell, io::BufReader, rc::Rc};
use windows::core::{implement, Interface, GUID};

mod registry;
mod winstream;
use winstream::WinStream;

use windows as Windows;
use windows::Win32::{
    Foundation::*,
    Graphics::Imaging::*,
    System::Com::{CoCreateInstance, IStream, CLSCTX_INPROC_SERVER},
};

mod dll;
mod guid;

mod properties;

#[implement(Windows::Win32::Graphics::Imaging::IWICBitmapDecoder)]
#[derive(Default)]
pub struct JXLWICBitmapDecoder {
    decoded: RefCell<Option<Rc<RefCell<DecodeProgress>>>>,
}

impl JXLWICBitmapDecoder {
    pub const CLSID: GUID = GUID::from_u128(0x655896c6_b7d0_4d74_8afb_a02ece3f5e5a);
    pub const CONTAINER_ID: GUID = GUID::from_u128(0x81e337bc_c1d1_4dee_a17c_402041ba9b5e);
}

impl IWICBitmapDecoder_Impl for JXLWICBitmapDecoder {
    fn QueryCapability(&self, _pistream: &Option<IStream>) -> windows::core::Result<u32> {
        log::trace!("QueryCapability");
        Ok((WICBitmapDecoderCapabilityCanDecodeSomeImages.0
            | WICBitmapDecoderCapabilityCanDecodeAllImages.0) as u32)
    }

    fn Initialize(
        &self,
        pistream: &Option<IStream>,
        _cacheoptions: WICDecodeOptions,
    ) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapDecoder::Initialize");

        let stream = WinStream::from(pistream.to_owned().unwrap());
        let reader = BufReader::new(stream);

        let decoder = Decoder::new();

        let result = decoder.decode_buffer(reader).map_err(|err| {
            windows::core::Error::new(WINCODEC_ERR_BADIMAGE, format!("{:?}", err).as_str().into())
        })?;
        self.decoded.replace(Some(Rc::new(RefCell::new(result))));

        Ok(())
    }

    fn GetContainerFormat(&self) -> windows::core::Result<GUID> {
        log::trace!("JXLWICBitmapDecoder::GetContainerFormat");
        // Randomly generated
        Ok(Self::CONTAINER_ID)
    }

    fn GetDecoderInfo(&self) -> windows::core::Result<IWICBitmapDecoderInfo> {
        log::trace!("JXLWICBitmapDecoder::GetDecoderInfo");
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let component_info = factory.CreateComponentInfo(&Self::CLSID)?;
            component_info.cast()
        }
    }

    fn CopyPalette(&self, _pipalette: &Option<IWICPalette>) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapDecoder::CopyPalette");
        WINCODEC_ERR_PALETTEUNAVAILABLE.ok()
    }

    fn GetMetadataQueryReader(&self) -> windows::core::Result<IWICMetadataQueryReader> {
        log::trace!("JXLWICBitmapDecoder::GetMetadataQueryReader");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }

    fn GetPreview(&self) -> windows::core::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapDecoder::GetPreview");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }

    fn GetColorContexts(
        &self,
        _ccount: u32,
        _ppicolorcontexts: *mut Option<IWICColorContext>,
        _pcactualcount: *mut u32,
    ) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapDecoder::GetColorContexts");
        WINCODEC_ERR_UNSUPPORTEDOPERATION.ok()
    }

    fn GetThumbnail(&self) -> windows::core::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapDecoder::GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.ok().unwrap_err())
    }

    fn GetFrameCount(&self) -> windows::core::Result<u32> {
        if self.decoded.borrow().is_none() {
            return Err(WINCODEC_ERR_NOTINITIALIZED.ok().unwrap_err());
        }

        let frame_count = self
            .decoded
            .borrow()
            .as_ref()
            .unwrap()
            .borrow()
            .frames
            .len();
        log::trace!("JXLWICBitmapDecoder::GetFrameCount: {}", frame_count);
        Ok(frame_count as u32)
    }

    fn GetFrame(&self, index: u32) -> windows::core::Result<IWICBitmapFrameDecode> {
        if self.decoded.borrow().is_none() {
            return Err(WINCODEC_ERR_NOTINITIALIZED.ok().unwrap_err());
        }

        let basic_info = self.decoded.borrow().as_ref().unwrap().borrow().basic_info;
        log::trace!("[{}]: {:?}", index, basic_info);

        let frame_decode =
            JXLWICBitmapFrameDecode::new(self.decoded.borrow().to_owned().unwrap(), index as usize);
        Ok(frame_decode.into())
    }
}

#[implement(Windows::Win32::Graphics::Imaging::IWICBitmapFrameDecode)]
pub struct JXLWICBitmapFrameDecode {
    decoded: Rc<RefCell<DecodeProgress>>,
    index: usize,
}

impl JXLWICBitmapFrameDecode {
    pub fn new(decoded: Rc<RefCell<DecodeProgress>>, index: usize) -> Self {
        Self { decoded, index }
    }
}

#[allow(non_snake_case)]
#[allow(clippy::missing_safety_doc)]
impl IWICBitmapSource_Impl for JXLWICBitmapFrameDecode {
    fn GetSize(&self, puiwidth: *mut u32, puiheight: *mut u32) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::GetSize");
        unsafe {
            *puiwidth = self.decoded.borrow().basic_info.xsize;
            *puiheight = self.decoded.borrow().basic_info.ysize;
        }
        Ok(())
    }

    fn GetPixelFormat(&self) -> windows::core::Result<GUID> {
        log::trace!("JXLWICBitmapFrameDecode::GetPixelFormat");
        // TODO: Support HDR
        Ok(GUID_WICPixelFormat32bppRGBA)
    }

    fn GetResolution(&self, pdpix: *mut f64, pdpiy: *mut f64) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::GetResolution");
        // TODO: Does JXL have resolution info?
        unsafe {
            *pdpix = 96f64;
            *pdpiy = 96f64;
        }
        Ok(())
    }

    fn CopyPalette(&self, _pipalette: &Option<IWICPalette>) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::CopyPalette");
        WINCODEC_ERR_PALETTEUNAVAILABLE.ok()
    }

    fn CopyPixels(
        &self,
        prc: *const WICRect,
        _cbstride: u32,
        _cbbuffersize: u32,
        pbbuffer: *mut u8,
    ) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::CopyPixels");

        if prc.is_null() {
            return Err(E_INVALIDARG.ok().unwrap_err());
        }

        let prc = unsafe { prc.as_ref().unwrap() };
        log::trace!("JXLWICBitmapFrameDecode::CopyPixels::WICRect {:?}", prc);

        let basic_info = &self.decoded.borrow().basic_info;
        let data = &self.decoded.borrow().frames[self.index].data;

        for y in prc.Y..(prc.Y + prc.Height) {
            let src_offset = basic_info.xsize as i32 * 4 * y;
            let dst_offset = prc.Width * 4 * (y - prc.Y);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr().offset((src_offset + prc.X) as isize),
                    pbbuffer.offset(dst_offset as isize),
                    (prc.Width as usize) * 4,
                );
            }
        }

        Ok(())
    }
}

impl IWICBitmapFrameDecode_Impl for JXLWICBitmapFrameDecode {
    fn GetMetadataQueryReader(&self) -> windows::core::Result<IWICMetadataQueryReader> {
        log::trace!("JXLWICBitmapFrameDecode::GetMetadataQueryReader");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.ok().unwrap_err())
    }

    fn GetColorContexts(
        &self,
        _ccount: u32,
        _ppicolorcontexts: *mut Option<IWICColorContext>,
        pcactualcount: *mut u32,
    ) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapFrameDecode::GetColorContexts");
        unsafe {
            *pcactualcount = 0;
        }
        Ok(())
    }

    fn GetThumbnail(&self) -> windows::core::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapFrameDecode::GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.ok().unwrap_err())
    }
}
