use aead::stream::{Encryptor, StreamBE32, StreamPrimitive};
use aead::{rand_core::RngCore, stream::NewStream, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit};
use anyhow::Result;
use futures::executor;
use std::cell::RefCell;
use std::io::prelude::Write;
use std::pin::Pin;
use tokio::io::{AsyncWrite, AsyncWriteExt};

const MAX_SAFE_ENCRYPTION_SIZE: usize = 34_359_738_368; // 32 gigs, the GCM safe limit

/// A wrapper around a writer that encrypts the data as it is written.
/// Should not be used on files larger than 32 GB.
pub struct EncryptionWriter<W: AsyncWrite + Unpin> {
    /// Internal buffer, holds data to be encrypted in place
    buf: RefCell<Vec<u8>>,
    /// Writer to another file
    writer: RefCell<W>,
    /// Encryptor. This stores the key
    encryptor: RefCell<Encryptor<Aes256Gcm, StreamBE32<Aes256Gcm>>>,
    /// Counter of bytes written
    bytes_written: RefCell<usize>,
}

/// A wrapper around a writer that encrypts the data as it is written.
impl<W: AsyncWrite + Unpin> EncryptionWriter<W> {
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
        let encryptor =
            RefCell::new(StreamBE32::from_aead(cipher, nonce.as_ref().into()).encryptor());
        Self {
            buf: RefCell::new(Vec::new()),
            writer: writer.into(),
            encryptor,
            bytes_written: RefCell::new(0),
        }
    }

    /// Encrypt the data in the buffer and write it to the writer.
    pub async fn finish(mut self) -> Result<usize> {
        self.flush()?;
        // TODO (laudiacay): check this logic better, especially once your PR is merged on rustcrypto
        self.encryptor
            .into_inner()
            .encrypt_last_in_place(b"".as_ref(), &mut *self.buf.borrow_mut())
            .unwrap();
        executor::block_on(self.writer.borrow_mut().write_all(&self.buf.borrow_mut()))?;
        *self.bytes_written.borrow_mut() += self.buf.borrow().len();
        Ok(*self.bytes_written.borrow())
    }

    pub fn cipher_info(&self) -> String {
        "AES-256-GCM".to_string()
    }
}

/// Implement the Write trait for EncryptionWriter.
impl<W: AsyncWrite + Unpin> Write for EncryptionWriter<W> {
    // TODO (laudiacay): Can we implement buffering better with bufwriter? not sure how this scales?
    /// Write data to the buffer
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    /// Clear the buffer and encrypt the data in place.
    fn flush(&mut self) -> std::io::Result<()> {
        let self_pin = Pin::new(&mut *self);
        self_pin
            .encryptor
            .borrow_mut()
            .encrypt_next_in_place(b"", &mut *self_pin.buf.borrow_mut())
            .unwrap();
        // TODO (laudiacay): YIKES! is this what we want? block_on???
        executor::block_on(
            self_pin
                .writer
                .borrow_mut()
                .write_all(&self_pin.buf.borrow()),
        )?;
        executor::block_on(self_pin.writer.borrow_mut().flush())?;
        *self_pin.bytes_written.borrow_mut() += self_pin.buf.borrow().len();
        if *self.bytes_written.borrow() >= MAX_SAFE_ENCRYPTION_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                "File too large to encrypt",
            ));
        };
        self.buf.borrow_mut().clear();
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
