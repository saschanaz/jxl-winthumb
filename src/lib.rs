#![crate_type = "dylib"]
// https://github.com/microsoft/windows-rs/issues/1506
#![allow(clippy::not_unsafe_ptr_arg_deref)]
// TODO: Update windows-rs
#![allow(unused_must_use)]
#![allow(non_snake_case)]

use jxl_oxide::{FrameBuffer, JxlImage, PixelFormat};
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

type JxlImageFromWinStream = JxlImage<BufReader<WinStream>>;

pub struct DecodedResult {
    frames: Vec<Rc<FrameBuffer>>,
    pixel_format: PixelFormat,
    icc: Rc<Vec<u8>>,
    width: u32,
    height: u32,
}

#[implement(Windows::Win32::Graphics::Imaging::IWICBitmapDecoder)]
#[derive(Default)]
pub struct JXLWICBitmapDecoder {
    decoded: RefCell<Option<DecodedResult>>,
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

        let mut image = JxlImageFromWinStream::from_reader(reader).map_err(|err| {
            windows::core::Error::new(WINCODEC_ERR_BADIMAGE, format!("{:?}", err).as_str().into())
        })?;

        let mut frames: Vec<Rc<FrameBuffer>> = Vec::new();

        let mut load_all = || -> jxl_oxide::Result<usize> {
            loop {
                let load_result = image.render_next_frame()?;
                match load_result {
                    jxl_oxide::RenderResult::NoMoreFrames => {
                        return Ok(image.num_loaded_keyframes())
                    }
                    jxl_oxide::RenderResult::NeedMoreData => {
                        return Err(Box::new(std::io::Error::from(
                            std::io::ErrorKind::UnexpectedEof,
                        )))
                    }
                    jxl_oxide::RenderResult::Done(render) => {
                        frames.push(Rc::new(render.image()));
                    }
                }
            }
        };
        load_all().map_err(|err| {
            windows::core::Error::new(WINCODEC_ERR_BADIMAGE, format!("{:?}", err).as_str().into())
        })?;

        let (width, height, _left, _top) = image.image_header().metadata.apply_orientation(
            image.image_header().size.width,
            image.image_header().size.height,
            0,
            0,
            false,
        );

        self.decoded.replace(Some(DecodedResult {
            frames,
            pixel_format: image.pixel_format(),
            icc: Rc::new(image.rendered_icc()),
            width,
            height,
        }));

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
        // TODO
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
        ccount: u32,
        ppicolorcontexts: *mut Option<IWICColorContext>,
        pcactualcount: *mut u32,
    ) -> windows::core::Result<()> {
        let decoded_ref = self.decoded.borrow();
        if decoded_ref.is_none() {
            return WINCODEC_ERR_NOTINITIALIZED.ok();
        }
        let decoded = decoded_ref.as_ref().unwrap();

        log::trace!(
            "JXLWICBitmapDecoder::GetColorContexts {} {:?} {:?}",
            ccount,
            ppicolorcontexts,
            pcactualcount
        );
        // TODO: Proper color context
        unsafe {
            if !ppicolorcontexts.is_null() && ccount == 1 {
                ppicolorcontexts
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .expect("There should be a color context here")
                    .InitializeFromMemory(&decoded.icc[..])?;
            }
            if !pcactualcount.is_null() {
                *pcactualcount = 1;
            }
        }
        Ok(())
    }

    fn GetThumbnail(&self) -> windows::core::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapDecoder::GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.ok().unwrap_err())
    }

    fn GetFrameCount(&self) -> windows::core::Result<u32> {
        let decoded_ref = self.decoded.borrow();
        if decoded_ref.is_none() {
            return Err(WINCODEC_ERR_NOTINITIALIZED.ok().unwrap_err());
        }
        let frame_count = decoded_ref.as_ref().unwrap().frames.len();

        log::trace!("JXLWICBitmapDecoder::GetFrameCount: {}", frame_count);
        Ok(frame_count as u32)
    }

    fn GetFrame(&self, index: u32) -> windows::core::Result<IWICBitmapFrameDecode> {
        let decoded_ref = self.decoded.borrow();
        if decoded_ref.is_none() {
            return Err(WINCODEC_ERR_NOTINITIALIZED.ok().unwrap_err());
        }

        let decoded = decoded_ref.as_ref().unwrap();
        log::trace!("[{}/{}]", index, decoded.frames.len());

        if index >= decoded.frames.len() as u32 {
            return Err(WINCODEC_ERR_FRAMEMISSING.ok().unwrap_err());
        }

        let frame_decode = JXLWICBitmapFrameDecode::new(
            decoded.frames[index as usize].clone(),
            decoded.pixel_format,
            decoded.icc.clone(),
            decoded.width,
            decoded.height,
        );
        Ok(frame_decode.into())
    }
}

#[implement(Windows::Win32::Graphics::Imaging::IWICBitmapFrameDecode)]
pub struct JXLWICBitmapFrameDecode {
    frame: Rc<FrameBuffer>,
    pixel_format: PixelFormat,
    icc: Rc<Vec<u8>>,
    width: u32,
    height: u32,
}

impl JXLWICBitmapFrameDecode {
    pub fn new(
        frame: Rc<FrameBuffer>,
        pixel_format: PixelFormat,
        icc: Rc<Vec<u8>>,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            frame,
            pixel_format,
            icc,
            width,
            height,
        }
    }
}

#[allow(non_snake_case)]
#[allow(clippy::missing_safety_doc)]
impl IWICBitmapSource_Impl for JXLWICBitmapFrameDecode {
    fn GetSize(&self, puiwidth: *mut u32, puiheight: *mut u32) -> windows::core::Result<()> {
        log::trace!(
            "JXLWICBitmapFrameDecode::GetSize {}x{}",
            self.width,
            self.height
        );
        unsafe {
            *puiwidth = self.width;
            *puiheight = self.height;
        }
        Ok(())
    }

    fn GetPixelFormat(&self) -> windows::core::Result<GUID> {
        log::trace!("JXLWICBitmapFrameDecode::GetPixelFormat");
        // TODO: Support all formats
        match self.pixel_format {
            PixelFormat::Gray => Ok(GUID_WICPixelFormat32bppGrayFloat),
            PixelFormat::Graya => Err(windows::core::Error::new(
                WINCODEC_ERR_UNSUPPORTEDPIXELFORMAT,
                "Gray alpha image is currently not supported".into(),
            )),
            PixelFormat::Rgb => Ok(GUID_WICPixelFormat96bppRGBFloat),
            PixelFormat::Rgba => Ok(GUID_WICPixelFormat128bppRGBAFloat),
            jxl_oxide::PixelFormat::Cmyk | jxl_oxide::PixelFormat::Cmyka => {
                Err(windows::core::Error::new(
                    WINCODEC_ERR_BADIMAGE,
                    "Cmyk is currently not supported".into(),
                ))
            }
        }
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

        let channels = self.frame.channels();

        for y in prc.Y..(prc.Y + prc.Height) {
            let src_offset = self.width as i32 * channels as i32 * y;
            let dst_offset = prc.Width * 4 * (y - prc.Y);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.frame
                        .buf()
                        .as_ptr()
                        .offset((src_offset + prc.X) as isize),
                    (pbbuffer as *mut f32).offset(dst_offset as isize),
                    (prc.Width as usize) * channels,
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
        ccount: u32,
        ppicolorcontexts: *mut Option<IWICColorContext>,
        pcactualcount: *mut u32,
    ) -> windows::core::Result<()> {
        log::trace!(
            "JXLWICBitmapFrameDecode::GetColorContexts {} {:?} {:?}",
            ccount,
            ppicolorcontexts,
            pcactualcount
        );
        unsafe {
            if !ppicolorcontexts.is_null() && ccount == 1 {
                ppicolorcontexts
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .expect("There should be a color context here")
                    .InitializeFromMemory(&self.icc[..])?;
            }
            if !pcactualcount.is_null() {
                *pcactualcount = 1;
            }
        }
        Ok(())
    }

    fn GetThumbnail(&self) -> windows::core::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapFrameDecode::GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.ok().unwrap_err())
    }
}
