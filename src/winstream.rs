use std::io::{ErrorKind, Read};
use winapi::um::objidlbase::{IStream, LPSTREAM};

pub struct WinStream {
    lpstream: LPSTREAM,
}

impl WinStream {
    pub fn new(lpstream: LPSTREAM) -> Self {
        Self { lpstream }
    }

    fn as_mut(&self) -> &mut IStream {
        unsafe { self.lpstream.as_mut().unwrap() }
    }
}

impl Read for WinStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let mut bytes_read = 0u32;
        let result = unsafe {
            self.as_mut()
                .Read(buf.as_mut_ptr() as _, buf.len() as u32, &mut bytes_read)
        };
        if result < 0 {
            Err(std::io::Error::new(
                ErrorKind::Other,
                format!("IStream::Read failed: {}", result),
            ))
        } else {
            Ok(bytes_read as usize)
        }
    }
}
