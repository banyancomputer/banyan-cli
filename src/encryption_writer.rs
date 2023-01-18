use aead::stream::{Encryptor, StreamBE32, StreamPrimitive};
use aead::{rand_core::RngCore, stream::NewStream, OsRng};
use aes_gcm::{Aes256Gcm, KeyInit};
use anyhow::Result;
use std::io::prelude::Write;

/// A wrapper around a writer that encrypts the data as it is written.
/// Should not be used on files larger than 32 GB.
pub struct EncryptionWriter<W: Write> {
    buf: Vec<u8>,
    writer: W,
    encryptor: Encryptor<Aes256Gcm, StreamBE32<Aes256Gcm>>,
    bytes_written: usize,
}

/// A wrapper around a writer that encrypts the data as it is written.
impl<W: Write> EncryptionWriter<W> {
    /// Create a new EncryptionWriter.
    ///
    /// # Arguments
    /// writer: The writer to write encrypted data to.
    /// key: The key to use for encryption.
    pub fn new(writer: W, key: &[u8]) -> Self {
        // Generate a random nonce.
        // TODO (laudiacay): Do we need to keep track of the nonce?
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

/// Implement the Write trait for EncryptionWriter.
impl<W: Write> Write for EncryptionWriter<W> {
    // TODO (laudiacay): Can we implement buffering better with bufwriter?
    /// Write data to the buffer
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }
    /// Clear the buffer and encrypt the data in place.
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

// TODO (xBalbinus & thea-exe): Our inline tests
#[cfg(test)]
mod test {
    #[test]
    fn test() {
        todo!()
    }
}
