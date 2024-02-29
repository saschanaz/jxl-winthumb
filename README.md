# jxl-winthumb

A JPEG XL (*.jxl) WIC decoder to render thumbnails on Windows File Explorer or view images on any WIC-capable image viewers.

## How to install

1. Download the dll file from https://github.com/saschanaz/jxl-winthumb/releases
1. Open a terminal window as administrator
1. Move to your download directory
1. `regsvr32 jxl_winthumb.dll`, or to uninstall, `regsvr32 /u jxl_winthumb.dll`.

You might need to restart `explorer.exe` for the component to work.  
You can use this command: `taskkill /f /im explorer.exe && start explorer.exe`

## Build environment

Use the stable Rust toolchain. Current toolchain as of 26th February 2024 is 1.75.0.

## Helpful resources

* [Integration with Windows Photo Gallery and Windows Explorer](https://docs.microsoft.com/en-us/windows/win32/wic/-wic-integrationregentries)
* [Using Kind Names](https://docs.microsoft.com/en-us/windows/win32/properties/building-property-handlers-user-friendly-kind-names)

## Inspired by

* [Intercom thumbnail provider example](https://github.com/Rantanen/intercom/tree/88d6a3c0470150805740b75ed23ec15131ec7469/samples/thumbnail_provider)
* [FlifWICCodec](https://github.com/peirick/FlifWICCodec/)
* [flif_windows_plugin](https://github.com/fherzog2/flif_windows_plugin/)
* [jpegxl-wic](https://github.com/mirillis/jpegxl-wic)
