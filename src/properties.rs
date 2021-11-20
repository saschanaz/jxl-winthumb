use std::io::BufReader;

use windows as Windows;
use windows::core::{implement, Interface, GUID};
use windows::Win32::{
    Foundation::*,
    System::Com::IStream,
    System::Com::StructuredStorage::PROPVARIANT,
    UI::Shell::PropertiesSystem::{
        IPropertyStoreCache, InitPropVariantFromStringVector, InitPropVariantFromUInt32Vector,
        PSCreateMemoryPropertyStore, PROPERTYKEY, PSC_READONLY,
    },
};

use crate::guid::get_guid_from_u128;
use crate::winstream::WinStream;
use kagamijxl::Decoder;

#[implement(
    Windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithStream,
    Windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore,
    Windows::Win32::UI::Shell::PropertiesSystem::IPropertyStoreCapabilities
)]
#[derive(Default)]
pub struct JXLPropertyStore {
    props: Option<IPropertyStoreCache>,
}

#[allow(non_snake_case)]
#[allow(clippy::missing_safety_doc)]
impl JXLPropertyStore {
    pub const CLSID: GUID = get_guid_from_u128(0x95ffe0f8_ab15_4751_a2f3_cfafdbf13664);

    pub unsafe fn Initialize(
        &self,
        pstream: &Option<IStream>,
        _grfmode: u32,
    ) -> windows::core::Result<()> {
        let stream = WinStream::from(pstream.to_owned().unwrap());
        let reader = BufReader::new(stream);

        let mut decoder = Decoder::new();
        decoder.no_full_image = true;
        let result = decoder.decode_buffer(reader).map_err(|err| {
            windows::core::Error::new(WINCODEC_ERR_BADIMAGE, format!("{:?}", err).as_str().into())
        })?;

        PSCreateMemoryPropertyStore(&IPropertyStoreCache::IID, std::mem::transmute(&self.props))?;

        let props = self.props.as_ref().unwrap();

        // XXX: This is copied from um/propkey.h.
        // https://github.com/microsoft/win32metadata/issues/730
        let PSGUID_IMAGESUMMARYINFORMATION =
            get_guid_from_u128(0x6444048F_4C8B_11D1_8B70_080036B11A03);

        let variant = InitPropVariantFromUInt32Vector(&result.basic_info.xsize, 1)?;
        let propkey = PROPERTYKEY {
            fmtid: PSGUID_IMAGESUMMARYINFORMATION,
            pid: 3,
        };
        props.SetValueAndState(&propkey, &variant, PSC_READONLY)?;

        let variant = InitPropVariantFromUInt32Vector(&result.basic_info.ysize, 1)?;
        let propkey = PROPERTYKEY {
            fmtid: PSGUID_IMAGESUMMARYINFORMATION,
            pid: 4,
        };
        props.SetValueAndState(&propkey, &variant, PSC_READONLY)?;

        // XXX: https://github.com/microsoft/windows-rs/issues/1288
        unsafe fn InitPropVariantFromStringVectorWrapped<
            'a,
            Param0: windows::core::IntoParam<'a, PWSTR>,
        >(
            pwstr: Param0,
        ) -> ::windows::core::Result<PROPVARIANT> {
            InitPropVariantFromStringVector(&pwstr.into_param().abi(), 1)
        }
        let variant = InitPropVariantFromStringVectorWrapped(format!(
            "{} x {}",
            result.basic_info.xsize, result.basic_info.ysize
        ))?;
        let propkey = PROPERTYKEY {
            fmtid: PSGUID_IMAGESUMMARYINFORMATION,
            pid: 13,
        };
        props.SetValueAndState(&propkey, &variant, PSC_READONLY)?;

        Ok(())
    }

    fn get_props(&self) -> windows::core::Result<&IPropertyStoreCache> {
        if self.props.is_none() {
            return Err(windows::core::Error::new(
                WINCODEC_ERR_NOTINITIALIZED,
                "Property store not initialized".into(),
            ));
        }

        Ok(self.props.as_ref().unwrap())
    }

    pub unsafe fn GetCount(&self) -> windows::core::Result<u32> {
        self.get_props()?.GetCount()
    }

    pub unsafe fn GetAt(&self, iprop: u32) -> windows::core::Result<PROPERTYKEY> {
        self.get_props()?.GetAt(iprop)
    }

    pub unsafe fn GetValue(&self, key: *const PROPERTYKEY) -> windows::core::Result<PROPVARIANT> {
        self.get_props()?.GetValue(key)
    }

    pub unsafe fn SetValue(
        &self,
        _key: *const PROPERTYKEY,
        _propvar: *const PROPVARIANT,
    ) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            WINCODEC_ERR_UNSUPPORTEDOPERATION,
            "Setter not supported".into(),
        ))
    }

    pub unsafe fn Commit(&self) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            WINCODEC_ERR_UNSUPPORTEDOPERATION,
            "Setter not supported".into(),
        ))
    }

    pub unsafe fn IsPropertyWritable(&self, _key: *const PROPERTYKEY) -> windows::core::Result<()> {
        Err(windows::core::Error::new(
            WINCODEC_ERR_UNSUPPORTEDOPERATION,
            "Setter not supported".into(),
        ))
    }
}
