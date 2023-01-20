use aead::stream::{Encryptor, StreamBE32, StreamPrimitive};
use aead::{rand_core::RngCore, stream::NewStream, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit};
use anyhow::{anyhow, Result};
use std::io::prelude::Write;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncWrite, AsyncWriteExt};

const MAX_SAFE_ENCRYPTION_SIZE: usize = 34_359_738_368; // 32 gigs, the GCM safe limit

/// A wrapper around a writer that encrypts the data as it is written.
/// Should not be used on files larger than 32 GB.
pub struct EncryptionWriter<W: AsyncWrite> {
    /// Internal buffer, holds data to be encrypted in place
    buf: Vec<u8>,
    /// Writer to another file
    writer: W,
    /// Encryptor. This stores the key
    encryptor: Encryptor<Aes256Gcm, StreamBE32<Aes256Gcm>>,
    /// Counter of bytes written
    bytes_written: usize,
}

/// A wrapper around a writer that encrypts the data as it is written.
impl<W: AsyncWrite> EncryptionWriter<W> {
    /// Create a new EncryptionWriter.
    ///
    /// # Arguments
    /// writer: The writer to write encrypted data to.
    /// key: The key to use for encryption.
    pub fn new(writer: W, key: &[u8]) -> Self {
        // Generate a random nonce.
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        // Create the encryptor.
        let cipher = Aes256Gcm::new(key.as_ref().into());
        let encryptor = StreamBE32::from_aead(cipher, nonce.as_ref().into()).encryptor();
        Self {
            buf: Vec::new(),
            writer,
            encryptor,
            bytes_written: 0,
        }
    }

    /// Encrypt the data in the buffer and write it to the writer.
    pub async fn finish(mut self) -> Result<usize> {
        self.flush()?;
        // TODO (laudiacay): check this logic better, especially once your PR is merged on rustcrypto
        self.encryptor
            .encrypt_last_in_place(b"".as_ref(), &mut self.buf)
            .unwrap();
        Box::pin(self.writer).write_all(&self.buf)?;
        self.bytes_written += self.buf.len();
        Ok(self.bytes_written)
    }

    pub fn cipher_info(self) -> String {
        "AES-256-GCM".to_string()
    }
}

/// Implement the Write trait for EncryptionWriter.
impl<W: AsyncWrite> Write for EncryptionWriter<W> {
    // TODO (laudiacay): Can we implement buffering better with bufwriter? not sure how this scales?
    /// Write data to the buffer
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    /// Clear the buffer and encrypt the data in place.
    fn flush(&mut self) -> std::io::Result<()> {
        let self_pin = Pin::new(&mut **self);
        self_pin
            .encryptor
            .encrypt_next_in_place(b"", &mut self.buf)
            .unwrap();
        self_pin.writer.write_all(&self.buf)?;
        self_pin.writer.flush()?;
        self_pin.bytes_written += self.buf.len();
        if self.bytes_written >= MAX_SAFE_ENCRYPTION_SIZE {
            return Err(std::io::Error::new(std::io::ErrorKind::FileTooLarge, "File too large to encrypt"))
        };
        self.buf.clear();
        Ok(())
    }
}

// TODO (xBalbinus & thea-exe): Our inline tests
#[cfg(test)]
mod test {
    #[test]
    fn test() {
        todo!()
    }
}
