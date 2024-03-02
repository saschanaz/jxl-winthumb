use std::io::BufReader;

use windows as Windows;
use windows::core::{implement, Interface, GUID, HSTRING, PCWSTR, PROPVARIANT};
use windows::Win32::{
    Foundation::*,
    System::Com::{
        IStream,
        StructuredStorage::{InitPropVariantFromStringVector, InitPropVariantFromUInt32Vector},
    },
    UI::Shell::PropertiesSystem::{
        IInitializeWithStream_Impl, IPropertyStoreCache, IPropertyStoreCapabilities_Impl,
        IPropertyStore_Impl, PSCreateMemoryPropertyStore, PROPERTYKEY, PSC_READONLY,
    },
};

use crate::winstream::WinStream;
use jxl_oxide::JxlImage;

#[implement(
    Windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream,
    Windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore,
    Windows::Win32::UI::Shell::PropertiesSystem::IPropertyStoreCapabilities
)]
#[derive(Default)]
pub struct JXLPropertyStore {
    props: Option<IPropertyStoreCache>,
}

impl JXLPropertyStore {
    pub const CLSID: GUID = GUID::from_u128(0x95ffe0f8_ab15_4751_a2f3_cfafdbf13664);

    fn get_props(&self) -> windows::core::Result<&IPropertyStoreCache> {
        match self.props {
            Some(ref props) => Ok(props),
            None => Err(windows::core::Error::new(
                WINCODEC_ERR_NOTINITIALIZED,
                "Property store not initialized",
            )),
        }
    }
}

impl IInitializeWithStream_Impl for JXLPropertyStore {
    fn Initialize(&self, pstream: Option<&IStream>, _grfmode: u32) -> windows::core::Result<()> {
        let stream = WinStream::from(pstream.unwrap());
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

        unsafe {
            PSCreateMemoryPropertyStore(
                &IPropertyStoreCache::IID,
                &self.props as *const _ as *mut *mut std::ffi::c_void,
            )?
        };

        let Some(props) = self.props.as_ref() else {
            return Err(windows::core::Error::new(
                WINCODEC_ERR_NOTINITIALIZED,
                "Property store not initialized",
            ));
        };

        // XXX: This is copied from um/propkey.h.
        // https://github.com/microsoft/win32metadata/issues/730
        #[allow(non_snake_case)]
        let PSGUID_IMAGESUMMARYINFORMATION =
            GUID::from_u128(0x6444048F_4C8B_11D1_8B70_080036B11A03);

        let variant = unsafe { InitPropVariantFromUInt32Vector(Some(&[width]))? };
        let propkey = PROPERTYKEY {
            fmtid: PSGUID_IMAGESUMMARYINFORMATION,
            pid: 3,
        };
        unsafe { props.SetValueAndState(&propkey, &variant, PSC_READONLY)? };

        let variant = unsafe { InitPropVariantFromUInt32Vector(Some(&[height]))? };
        let propkey = PROPERTYKEY {
            fmtid: PSGUID_IMAGESUMMARYINFORMATION,
            pid: 4,
        };
        unsafe { props.SetValueAndState(&propkey, &variant, PSC_READONLY)? };

        let variant = unsafe {
            InitPropVariantFromStringVector(Some(&[PCWSTR(
                HSTRING::from(format!("{} x {}", width, height)).as_ptr(),
            )]))?
        };
        let propkey = PROPERTYKEY {
            fmtid: PSGUID_IMAGESUMMARYINFORMATION,
            pid: 13,
        };
        unsafe { props.SetValueAndState(&propkey, &variant, PSC_READONLY)? };

        Ok(())
    }
}

impl IPropertyStore_Impl for JXLPropertyStore {
    fn GetCount(&self) -> windows::core::Result<u32> {
        unsafe { self.get_props()?.GetCount() }
    }

    fn GetAt(&self, iprop: u32, pkey: *mut PROPERTYKEY) -> windows::core::Result<()> {
        unsafe {
            self.get_props()?.GetAt(iprop, pkey);
        }
        Ok(())
    }

    fn GetValue(&self, key: *const PROPERTYKEY) -> windows::core::Result<PROPVARIANT> {
        unsafe { self.get_props()?.GetValue(key) }
    }

    fn SetValue(
        &self,
        _key: *const PROPERTYKEY,
        _propvar: *const PROPVARIANT,
    ) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            WINCODEC_ERR_UNSUPPORTEDOPERATION,
            "Setter not supported",
        ))
    }

    fn Commit(&self) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            WINCODEC_ERR_UNSUPPORTEDOPERATION,
            "Setter not supported",
        ))
    }
}

impl IPropertyStoreCapabilities_Impl for JXLPropertyStore {
    fn IsPropertyWritable(&self, _key: *const PROPERTYKEY) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            WINCODEC_ERR_UNSUPPORTEDOPERATION,
            "Setter not supported",
        ))
    }
}
