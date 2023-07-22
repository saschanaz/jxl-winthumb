use std::io::BufReader;

use windows as Windows;
use windows::core::{implement, Interface, GUID, PCWSTR, PWSTR};
use windows::Win32::{
    Foundation::*,
    System::Com::IStream,
    System::Com::StructuredStorage::PROPVARIANT,
    UI::Shell::PropertiesSystem::{
        IInitializeWithStream_Impl, IPropertyStoreCache, IPropertyStoreCapabilities_Impl,
        IPropertyStore_Impl, InitPropVariantFromStringVector, InitPropVariantFromUInt32Vector,
        PSCreateMemoryPropertyStore, PROPERTYKEY, PSC_READONLY,
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
        if self.props.is_none() {
            return Err(windows::core::Error::new(
                WINCODEC_ERR_NOTINITIALIZED,
                "Property store not initialized".into(),
            ));
        }

        Ok(self.props.as_ref().unwrap())
    }
}

impl IInitializeWithStream_Impl for JXLPropertyStore {
    fn Initialize(&self, pstream: &Option<IStream>, _grfmode: u32) -> windows::core::Result<()> {
        let stream = WinStream::from(pstream.to_owned().unwrap());
        let reader = BufReader::new(stream);

        let decoder = JxlImage::from_reader(reader).map_err(|err| {
            windows::core::Error::new(WINCODEC_ERR_BADIMAGE, format!("{:?}", err).as_str().into())
        })?;

        let size = &decoder.image_header().size;

        unsafe {
            PSCreateMemoryPropertyStore(
                &IPropertyStoreCache::IID,
                std::mem::transmute(&self.props),
            )?
        };

        let props = self.props.as_ref().unwrap();

        // XXX: This is copied from um/propkey.h.
        // https://github.com/microsoft/win32metadata/issues/730
        #[allow(non_snake_case)]
        let PSGUID_IMAGESUMMARYINFORMATION =
            GUID::from_u128(0x6444048F_4C8B_11D1_8B70_080036B11A03);

        let variant = unsafe { InitPropVariantFromUInt32Vector(&[size.width])? };
        let propkey = PROPERTYKEY {
            fmtid: PSGUID_IMAGESUMMARYINFORMATION,
            pid: 3,
        };
        unsafe { props.SetValueAndState(&propkey, &variant, PSC_READONLY)? };

        let variant = unsafe { InitPropVariantFromUInt32Vector(&[size.height])? };
        let propkey = PROPERTYKEY {
            fmtid: PSGUID_IMAGESUMMARYINFORMATION,
            pid: 4,
        };
        unsafe { props.SetValueAndState(&propkey, &variant, PSC_READONLY)? };

        // XXX: https://github.com/microsoft/windows-rs/issues/1288
        #[allow(non_snake_case)]
        unsafe fn InitPropVariantFromStringVectorWrapped<
            'a,
            Param0: windows::core::IntoParam<'a, PCWSTR>,
        >(
            pcwstr: Param0,
        ) -> ::windows::core::Result<PROPVARIANT> {
            let pcwstr = pcwstr.into_param().abi();
            InitPropVariantFromStringVector(&[PWSTR(pcwstr.0 as *mut _)])
        }
        let variant = unsafe {
            InitPropVariantFromStringVectorWrapped(format!("{} x {}", size.width, size.height))?
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

    fn GetAt(&self, iprop: u32) -> windows::core::Result<PROPERTYKEY> {
        unsafe { self.get_props()?.GetAt(iprop) }
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
            "Setter not supported".into(),
        ))
    }

    fn Commit(&self) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            WINCODEC_ERR_UNSUPPORTEDOPERATION,
            "Setter not supported".into(),
        ))
    }
}

impl IPropertyStoreCapabilities_Impl for JXLPropertyStore {
    fn IsPropertyWritable(&self, _key: *const PROPERTYKEY) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            WINCODEC_ERR_UNSUPPORTEDOPERATION,
            "Setter not supported".into(),
        ))
    }
}
