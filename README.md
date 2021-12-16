# jxl-winthumb

A JPEG XL (*.jxl) thumbnail handler for Windows File Explorer.

Now with WIC decoding support, which means you can use Windows Photo Viewer or any WIC-capable image viewers to view JXL files.

## How to install

0. Install [Microsoft Visual C++ Redistributable for Visual Studio 2022](https://visualstudio.microsoft.com/downloads/#microsoft-visual-c-redistributable-for-visual-studio-2022) if it's not already installed.
1. Download the latest dll file from the [releases](https://github.com/saschanaz/jxl-winthumb/releases) page.
2. Open a terminal window as administrator
3. Move to your download directory
4. `regsvr32 jxl_winthumb.dll`, or to uninstall, `regsvr32 /u jxl_winthumb.dll`.

## Build environment

Please read [the requirements](https://github.com/saschanaz/jxl-rs/tree/main/libjxl-src) to build the libjxl dependency, or take a look at [the CI configuration](https://github.com/saschanaz/jxl-winthumb/blob/main/.github/workflows/ci.yml).

## Helpful resources

* [Integration with Windows Photo Gallery and Windows Explorer](https://docs.microsoft.com/en-us/windows/win32/wic/-wic-integrationregentries)
* [Using Kind Names](https://docs.microsoft.com/en-us/windows/win32/properties/building-property-handlers-user-friendly-kind-names)

## Inspired by

* [Intercom thumbnail provider example](https://github.com/Rantanen/intercom/tree/88d6a3c0470150805740b75ed23ec15131ec7469/samples/thumbnail_provider)
* [FlifWICCodec](https://github.com/peirick/FlifWICCodec/)
* [flif_windows_plugin](https://github.com/fherzog2/flif_windows_plugin/)
* [jpegxl-wic](https://github.com/mirillis/jpegxl-wic)
