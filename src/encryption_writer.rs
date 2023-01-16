use anyhow::Result;

use std::io::prelude::Write;

use aead::stream::{Encryptor, StreamBE32, StreamPrimitive};
use aead::{rand_core::RngCore, stream::NewStream, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit};

pub struct EncryptionWriter<W: Write> {
    buf: Vec<u8>,
    writer: W,
    encryptor: Encryptor<Aes256Gcm, StreamBE32<Aes256Gcm>>,
    bytes_written: usize,
}

impl<W: Write> EncryptionWriter<W> {
    pub fn new(writer: W, key: &[u8]) -> Self {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let cipher = Aes256Gcm::new(key.as_ref().into());
        let encryptor = StreamBE32::from_aead(cipher, nonce.as_ref().into()).encryptor();
        Self {
            buf: Vec::new(),
            writer,
            encryptor,
            bytes_written: 0,
        }
    }

    pub fn finish(mut self) -> Result<usize> {
        self.flush()?;
        self.encryptor
            .encrypt_last_in_place(b"".as_ref(), &mut self.buf)
            .unwrap();
        self.writer.write_all(&self.buf)?;
        self.bytes_written += self.buf.len();
        Ok(self.bytes_written)
    }
}

impl<W: Write> Write for EncryptionWriter<W> {
    // TODO better buffering? bufwriter?
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.encryptor
            .encrypt_next_in_place(b"", &mut self.buf)
            .unwrap();
        self.writer.write_all(&self.buf)?;
        self.writer.flush()?;
        self.bytes_written += self.buf.len();
        self.buf.clear();
        Ok(())
    }
}
