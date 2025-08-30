use std::io::Read;

use windows::Win32::System::Com::IStream;

pub struct WinStream<'a> {
    stream: &'a IStream,
}

impl<'a> From<&'a IStream> for WinStream<'a> {
    fn from(stream: &'a IStream) -> Self {
        Self { stream }
    }
}

impl Read for WinStream<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut bytes_read = 0u32;
        unsafe {
            self.stream.Read(
                buf.as_mut_ptr() as _,
                buf.len() as u32,
                Some((&mut bytes_read) as *mut _),
            )
        }
        .ok()
        .map_err(|err| std::io::Error::other(format!("IStream::Read failed: {}", err.code().0)))?;
        Ok(bytes_read as usize)
    }
}
