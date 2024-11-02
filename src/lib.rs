#![crate_type = "dylib"]
// https://github.com/microsoft/windows-rs/issues/1506
#![allow(clippy::not_unsafe_ptr_arg_deref)]
// TODO: Update windows-rs
#![allow(unused_must_use)]
#![allow(non_snake_case)]

use jxl_oxide::{JxlImage, PixelFormat};
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

pub struct DecodedResult {
    image: JxlImage,
    frame_count: usize,
    pixel_format: PixelFormat,
    icc: Rc<Vec<u8>>,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
pub struct FrameBuffer {
    pub channels: usize,
    pub buf: Vec<u16>,
}

impl FrameBuffer {
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Self {
            channels,
            buf: vec![0u16; width * height * channels],
        }
    }
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

impl IWICBitmapDecoder_Impl for JXLWICBitmapDecoder_Impl {
    fn QueryCapability(&self, _pistream: Option<&IStream>) -> windows::core::Result<u32> {
        log::trace!("QueryCapability");
        Ok((WICBitmapDecoderCapabilityCanDecodeSomeImages.0
            | WICBitmapDecoderCapabilityCanDecodeAllImages.0) as u32)
    }

    fn Initialize(
        &self,
        pistream: Option<&IStream>,
        _cacheoptions: WICDecodeOptions,
    ) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapDecoder::Initialize");

        let stream = WinStream::from(pistream.unwrap());
        let reader = BufReader::new(stream);

        let image = JxlImage::builder().read(reader).map_err(|err| {
            windows::core::Error::new(WINCODEC_ERR_BADIMAGE, format!("{:?}", err))
        })?;

        let (width, height, _left, _top) = image.image_header().metadata.apply_orientation(
            image.image_header().size.width,
            image.image_header().size.height,
            0,
            0,
            false,
        );

        self.decoded.replace(Some(DecodedResult {
            frame_count: image.num_loaded_keyframes(),
            pixel_format: image.pixel_format(),
            icc: Rc::new(image.rendered_icc()),
            image,
            width,
            height,
        }));

        Ok(())
    }

    fn GetContainerFormat(&self) -> windows::core::Result<GUID> {
        log::trace!("JXLWICBitmapDecoder::GetContainerFormat");
        // Randomly generated
        Ok(JXLWICBitmapDecoder::CONTAINER_ID)
    }

    fn GetDecoderInfo(&self) -> windows::core::Result<IWICBitmapDecoderInfo> {
        log::trace!("JXLWICBitmapDecoder::GetDecoderInfo");
        unsafe {
            let factory: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
            let component_info = factory.CreateComponentInfo(&JXLWICBitmapDecoder::CLSID)?;
            component_info.cast()
        }
    }

    fn CopyPalette(&self, _pipalette: Option<&IWICPalette>) -> windows::core::Result<()> {
        log::trace!("JXLWICBitmapDecoder::CopyPalette");
        // TODO
        WINCODEC_ERR_PALETTEUNAVAILABLE.ok()
    }

    fn GetMetadataQueryReader(&self) -> windows::core::Result<IWICMetadataQueryReader> {
        log::trace!("JXLWICBitmapDecoder::GetMetadataQueryReader");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.into())
    }

    fn GetPreview(&self) -> windows::core::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapDecoder::GetPreview");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.into())
    }

    fn GetColorContexts(
        &self,
        ccount: u32,
        ppicolorcontexts: *mut Option<IWICColorContext>,
        pcactualcount: *mut u32,
    ) -> windows::core::Result<()> {
        let decoded_ref = self.decoded.borrow();

        let Some(decoded) = decoded_ref.as_ref() else {
            return WINCODEC_ERR_NOTINITIALIZED.ok();
        };

        log::trace!(
            "JXLWICBitmapDecoder::GetColorContexts {} {:?} {:?}",
            ccount,
            ppicolorcontexts,
            pcactualcount
        );
        // TODO: Proper color context
        unsafe {
            if let Some(context) = ppicolorcontexts.as_mut() {
                if ccount == 1 {
                    context
                        .as_mut()
                        .expect("There should be a color context here")
                        .InitializeFromMemory(&decoded.icc[..])?;
                }
            }
            if !pcactualcount.is_null() {
                *pcactualcount = 1;
            }
        }
        Ok(())
    }

    fn GetThumbnail(&self) -> windows::core::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapDecoder::GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.into())
    }

    fn GetFrameCount(&self) -> windows::core::Result<u32> {
        let decoded_ref = self.decoded.borrow();
        let Some(decoded) = decoded_ref.as_ref() else {
            return Err(WINCODEC_ERR_NOTINITIALIZED.into());
        };
        let frame_count = decoded.frame_count;

        log::trace!("JXLWICBitmapDecoder::GetFrameCount: {}", frame_count);
        Ok(frame_count as u32)
    }

    fn GetFrame(&self, index: u32) -> windows::core::Result<IWICBitmapFrameDecode> {
        let mut decoded_ref = self.decoded.borrow_mut();
        let Some(decoded) = decoded_ref.as_mut() else {
            return Err(WINCODEC_ERR_NOTINITIALIZED.into());
        };

        log::trace!("[{}/{}]", index, decoded.frame_count);

        if index >= decoded.frame_count as u32 {
            return Err(WINCODEC_ERR_FRAMEMISSING.into());
        }

        let render = decoded.image.render_frame(index as usize).map_err(|err| {
            windows::core::Error::new(WINCODEC_ERR_FRAMEMISSING, format!("{:?}", err))
        })?;

        let mut stream = render.stream();
        let mut fb = FrameBuffer::new(
            stream.width() as usize,
            stream.height() as usize,
            stream.channels() as usize,
        );
        stream.write_to_buffer(&mut fb.buf[..]);

        let frame_decode = JXLWICBitmapFrameDecode::new(
            fb,
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
    frame: FrameBuffer,
    pixel_format: PixelFormat,
    icc: Rc<Vec<u8>>,
    width: u32,
    height: u32,
}

impl JXLWICBitmapFrameDecode {
    pub fn new(
        frame: FrameBuffer,
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
impl IWICBitmapSource_Impl for JXLWICBitmapFrameDecode_Impl {
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

        match self.pixel_format {
            PixelFormat::Gray => Ok(GUID_WICPixelFormat16bppGray),
            // WIC doesn't support Graya, but maybe can be emulated with RGBA
            PixelFormat::Graya => Err(windows::core::Error::new(
                WINCODEC_ERR_UNSUPPORTEDPIXELFORMAT,
                "Gray alpha image is currently not supported",
            )),
            PixelFormat::Rgb => Ok(GUID_WICPixelFormat48bppRGB),
            PixelFormat::Rgba => Ok(GUID_WICPixelFormat64bppRGBA),
            PixelFormat::Cmyk => Ok(GUID_WICPixelFormat64bppCMYK),
            PixelFormat::Cmyka => Ok(GUID_WICPixelFormat80bppCMYKAlpha),
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

    fn CopyPalette(&self, _pipalette: Option<&IWICPalette>) -> windows::core::Result<()> {
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

        let pbbuffer = pbbuffer as *mut u16;

        let Some(prc) = (unsafe { prc.as_ref() }) else {
            return Err(E_INVALIDARG.into());
        };

        log::trace!("JXLWICBitmapFrameDecode::CopyPixels::WICRect {:?}", prc);

        let channels = self.frame.channels;
        let buf = &self.frame.buf;

        for y in prc.Y..(prc.Y + prc.Height) {
            let src_offset = (self.width as i32 * y + prc.X) * (channels as i32);
            let dst_offset = prc.Width * (y - prc.Y) * (channels as i32);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    buf.as_ptr().offset(src_offset as isize),
                    pbbuffer.offset(dst_offset as isize),
                    (prc.Width as usize) * channels,
                );
            }
        }

        Ok(())
    }
}

impl IWICBitmapFrameDecode_Impl for JXLWICBitmapFrameDecode_Impl {
    fn GetMetadataQueryReader(&self) -> windows::core::Result<IWICMetadataQueryReader> {
        log::trace!("JXLWICBitmapFrameDecode::GetMetadataQueryReader");
        Err(WINCODEC_ERR_UNSUPPORTEDOPERATION.into())
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
            if let Some(context) = ppicolorcontexts.as_mut() {
                if ccount == 1 {
                    context
                        .as_mut()
                        .expect("There should be a color context here")
                        .InitializeFromMemory(&self.icc[..])?;
                }
            }
            if !pcactualcount.is_null() {
                *pcactualcount = 1;
            }
        }
        Ok(())
    }

    fn GetThumbnail(&self) -> windows::core::Result<IWICBitmapSource> {
        log::trace!("JXLWICBitmapFrameDecode::GetThumbnail");
        Err(WINCODEC_ERR_CODECNOTHUMBNAIL.into())
    }
}
