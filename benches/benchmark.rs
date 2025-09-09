use criterion::{Criterion, criterion_group, criterion_main};
use jxl_winthumb::JXLWICBitmapDecoder;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::CoInitialize;
use windows::Win32::UI::Shell::SHCreateMemStream;

fn basic() {
    unsafe { CoInitialize(None) }.ok().expect("CoInitialize");

    let mem = std::fs::read("tests/alien.jxl").expect("Read the test file");
    let stream = unsafe { SHCreateMemStream(Some(&mem[..])) }.expect("Create an IStream");
    let decoder: IWICBitmapDecoder = JXLWICBitmapDecoder::default().into();
    unsafe { decoder.Initialize(&stream, WICDecodeOptions(0)) }.expect("Initialize the decoder");
    unsafe { decoder.GetFrame(0) }.expect("Get the first frame");
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("alien.jxl", |b| b.iter(basic));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
