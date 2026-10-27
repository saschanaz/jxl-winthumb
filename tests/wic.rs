use jxl_winthumb::{JXLPropertyStore, JXLWICBitmapDecoder};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoInitialize};
use windows::Win32::UI::Shell::PropertiesSystem::{IInitializeWithStream, IPropertyStore};
use windows::Win32::UI::Shell::{PSGUID_IMAGESUMMARYINFORMATION, SHCreateMemStream};
use windows::core::Interface;

#[test]
fn basic() {
    unsafe { CoInitialize(None) }.ok().expect("CoInitialize");

    let mem = std::fs::read("tests/alien.jxl").expect("Read the test file");
    let stream = unsafe { SHCreateMemStream(Some(&mem[..])) }.expect("Create an IStream");
    let decoder: IWICBitmapDecoder = JXLWICBitmapDecoder::default().into();
    unsafe { decoder.Initialize(&stream, WICDecodeOptions(0)) }.expect("Initialize the decoder");
    let frame = unsafe { decoder.GetFrame(0) }.expect("Get the first frame");
    let source = unsafe { WICConvertBitmapSource(&GUID_WICPixelFormat32bppPRGBA, &frame) }
        .expect("Create a bitmap source");

    let factory: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
            .expect("Create a factory");
    let bitmap = unsafe { factory.CreateBitmapFromSource(&source, WICBitmapCacheOnDemand) }
        .expect("Create a bitmap");

    let mut width = 0u32;
    let mut height = 0u32;
    unsafe { bitmap.GetSize(&mut width, &mut height).expect("GetSize") };
    assert_eq!(width, 1024, "width");
    assert_eq!(height, 1024, "height");

    let mut pixels: Vec<u8> = vec![0; 1024 * 1024 * 4];
    unsafe {
        bitmap.CopyPixels(
            &WICRect {
                X: 0,
                Y: 0,
                Width: 1024,
                Height: 1024,
            },
            1024 * 4,
            &mut pixels,
        )
    }
    .expect("Copy pixels");
    assert_eq!(pixels[0], 0, "red");
    assert_eq!(pixels[1], 6, "green");
    assert_eq!(pixels[2], 0, "blue");
    assert_eq!(pixels[3], 255, "alpha");
}

#[test]
fn property_store() {
    let mem = std::fs::read("tests/alien.jxl").expect("Read the test file");
    let stream = unsafe { SHCreateMemStream(Some(&mem[..])) }.expect("Create an IStream");

    let init: IInitializeWithStream = JXLPropertyStore::default().into();
    unsafe { init.Initialize(&stream, 0) }.expect("Initialize the property store");
    let store: IPropertyStore = init.cast().expect("Cast to IPropertyStore");

    let count = unsafe { store.GetCount() }.expect("GetCount");
    assert_eq!(
        count, 3,
        "Expected 3 properties (width, height, dimensions)"
    );

    let fmtid = PSGUID_IMAGESUMMARYINFORMATION;
    for (pid, expected, name) in [
        (3, "1024", "width"),
        (4, "1024", "height"),
        (13, "1024 x 1024", "dimensions"),
    ] {
        let value = unsafe { store.GetValue(&PROPERTYKEY { fmtid, pid }) }.expect("GetValue");
        assert_eq!(value.to_string(), expected, "Get {name} property");
    }
}
