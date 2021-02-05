# jxl-winthumb

A JPEG XL (*.jxl) thumbnail handler for Windows File Explorer.

## Why not Windows Imaging Component?

It was useful before Windows 10 era when Windows Photo Gallery existed, but not anymore as Microsoft now uses its own undocumented way to add system codecs for UWP apps.

## Build environment

Please read [the requirements](https://github.com/saschanaz/jxl-rs/tree/main/libjxl-src) to build the libjxl dependency, or take a look at [the CI configuration](https://github.com/saschanaz/jxl-winthumb/blob/main/.github/workflows/ci.yml).

## Helpful resources

* [Thumbnail Handlers](https://docs.microsoft.com/en-us/windows/win32/shell/thumbnail-providers)
* [IThumbnailProvider::GetThumbnail method (thumbcache.h)](https://docs.microsoft.com/en-us/windows/win32/api/thumbcache/nf-thumbcache-ithumbnailprovider-getthumbnail)
* [IInitializeWithStream::Initialize method (propsys.h)](https://docs.microsoft.com/en-us/windows/win32/api/propsys/nf-propsys-iinitializewithstream-initialize)

## Inspired by

* [Intercom thumbnail provider example](https://github.com/Rantanen/intercom/tree/88d6a3c0470150805740b75ed23ec15131ec7469/samples/thumbnail_provider)
* [FlifWICCodec](https://github.com/peirick/FlifWICCodec/)
