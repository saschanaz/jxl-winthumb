use std::io::{ErrorKind, Read};

use windows::Win32::System::Com::IStream;

pub struct WinStream {
    stream: IStream,
}

impl From<IStream> for WinStream {
    fn from(stream: IStream) -> Self {
        Self { stream }
    }
}

impl Read for WinStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut bytes_read = 0u32;
        unsafe {
            self.stream
                .Read(buf.as_mut_ptr() as _, buf.len() as u32, &mut bytes_read)
        }
        .ok()
        .map_err(|err| {
            std::io::Error::new(
                ErrorKind::Other,
                format!("IStream::Read failed: {}", err.code().0),
            )
        })?;
        Ok(bytes_read as usize)
    }
}
