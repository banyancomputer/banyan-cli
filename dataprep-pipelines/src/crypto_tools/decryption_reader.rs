use crate::crypto_tools::key_and_nonce_types::{KeyAndNonce, KeyAndNonceToDisk};
use aead::stream::{Decryptor, NewStream, StreamBE32, StreamPrimitive};
use aes_gcm::{Aes256Gcm, KeyInit};
use anyhow::{anyhow, Result};
use futures::executor;
use std::cell::RefCell;
use std::io::{Cursor, ErrorKind, Read, Write};
use std::pin::Pin;

const BUF_SIZE: usize = 1024 * 1024; // 1 MB

/// A wrapper around a writer that encrypts the data as it is written.
/// Should not be used on files larger than 32 GB.
pub struct DecryptionReader<R: Read + Unpin> {
    /// Internal buffer, holds data from disk and decrypts it in place
    /// // TODO why a vec? why not a fixed size array?
    buf: RefCell<Vec<u8>>,
    /// buf_ptr tracks the current position in the buffer- where to start reading decrypted data from
    buf_ptr: RefCell<usize>,
    /// bytes_in_buffer tracks how many bytes are in the buffer- it may not always be full
    bytes_in_buffer: RefCell<usize>,
    /// Writer to another file
    reader: RefCell<R>,
    /// Decryptor. This stores the key
    decryptor: RefCell<Decryptor<Aes256Gcm, StreamBE32<Aes256Gcm>>>,
    /// Counter of bytes read into the read() method
    bytes_read: RefCell<usize>,
    /// size limit for buffer
    // TODO implement me
    _size_limit: usize,
    /// eof checker
    eof: RefCell<bool>,
}

/// A wrapper around a reader that decrypts the data as it is written.
impl<R: Read + Unpin> DecryptionReader<R> {
    /// Create a new DecryptionReader.
    ///
    /// # Arguments
    /// reader: The reader to read encrypted data from. should start with the nonce.
    /// key_and_nonce: The key and nonce to use for decryption.
    ///
    /// # Returns
    /// A DecryptionReader
    pub async fn new(reader: R, key_and_nonce: KeyAndNonceToDisk) -> Result<Self> {
        let KeyAndNonce { key, nonce } = *key_and_nonce.consume_and_prep_from_disk()?;
        let cipher = Aes256Gcm::new(&key);
        let decryptor = RefCell::new(StreamBE32::from_aead(cipher, &nonce).decryptor());

        Ok(Self {
            buf: RefCell::new(Vec::new()),
            bytes_in_buffer: RefCell::new(0),
            buf_ptr: RefCell::new(0),
            reader: reader.into(),
            decryptor,
            _size_limit: BUF_SIZE, // TODO (laudiacay) maybe one day make changeable
            bytes_read: RefCell::new(0),
            eof: RefCell::new(false),
        })
    }

    /// Read from the Reader into the buffer, decrypting it in place.
    pub async fn refresh_buffer(&mut self) -> std::io::Result<()> {
        // Borrow the mutable references to the fields we need
        let mut buf = self.buf.borrow_mut();
        let mut reader = self.reader.borrow_mut();
        let mut decryptor = self.decryptor.borrow_mut();
        let mut buf_ptr = self.buf_ptr.borrow_mut();
        let mut bytes_in_buffer = self.bytes_in_buffer.borrow_mut();
        let mut eof = self.eof.borrow_mut();
        // Assert that our internal buffer is empty
        assert_eq!(*buf_ptr, *bytes_in_buffer);
        // Clear the buffer
        buf.clear();
        // Make sure the buffer is the right size (Do we need this?)
        buf.resize(BUF_SIZE, 0);
        // Read from the reader into the buffer
        let new_bytes_read = reader.read(&mut buf)?;
        // If we read 0 bytes, we're at the end of the file
        if new_bytes_read == 0 {
            // Set eof to true
            *eof = true;
        }
        // Otherwise, there's more data to decrypt
        else {
            (*decryptor)
                .decrypt_next_in_place(b"".as_ref(), &mut *buf)
                .map_err(|_| {
                    std::io::Error::new(ErrorKind::Other, anyhow!("Error decrypting block!"))
                })?;
        };
        // Update the buffer pointer and bytes in buffer
        // Set the bytes in buffer to the number of bytes read
        *bytes_in_buffer = new_bytes_read;
        // Set the buffer pointer to 0
        *buf_ptr = 0;
        Ok(())
    }

    pub async fn finish(self) -> Result<usize> {
        assert_eq!(
            self.reader.borrow_mut().read(&mut self.buf.borrow_mut())?,
            0
        );
        *self.bytes_read.borrow_mut() += self.buf.borrow().len();
        self.decryptor
            .into_inner()
            .decrypt_last_in_place(b"".as_ref(), &mut *self.buf.borrow_mut())
            .map_err(|_| anyhow!("Error decrypting last block"))?;
        Ok(*self.bytes_read.borrow_mut())
    }

    pub fn cipher_info(&self) -> String {
        "AES-256-GCM".to_string()
    }
}

/// Implement the Read trait for DecryptionReader.
impl<R: Read + Unpin> Read for DecryptionReader<R> {
    /// Read from the DecryptionReader into a buffer.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Determine how many bytes can read into the buffer
        let buf_len = buf.len();
        let mut bytes_to_read = buf_len;
        // Declare a cursor to write into the buffer
        let mut buf_cursor = Cursor::new(buf);
        // Create a new pin to borrow the fields we need
        let mut self_pin = Pin::new(&mut *self);

        // Read as many bytes as we can into the buffer
        while bytes_to_read > 0 && !*self_pin.eof.borrow() {
            // There's still bytes to read, and we're not at the end of the file

            // Determine how many bytes are available in the internal buffer
            let bytes_available = *self_pin.bytes_in_buffer.borrow() - *self_pin.buf_ptr.borrow();
            // If there's more to read than there's space available, read the space available
            if bytes_available >= bytes_to_read {
                {
                    let buf_ptr = self_pin.buf_ptr.borrow();
                    // Copy the bytes that can be read into the output buffer
                    buf_cursor
                        .write_all(&self_pin.buf.borrow()[*buf_ptr..*buf_ptr + bytes_to_read])?;
                    // Move the buffer pointer to where we left off
                    *self_pin.buf_ptr.borrow_mut() += bytes_to_read;
                    // We've read all the bytes we can read
                    bytes_to_read = 0;
                }
            }
            // Otherwise, read the bytes available
            else {
                // Do we need to write if bytes_available == 0?
                {
                    let mut buf_ptr = self_pin.buf_ptr.borrow_mut();
                    // Copy all available bytes from the internal buffer into the output buffer
                    buf_cursor
                        .write_all(&self_pin.buf.borrow()[*buf_ptr..*buf_ptr + bytes_available])?;
                    // Update the buffer pointer (They should be equal now)
                    *buf_ptr += bytes_available;
                    // Update the number of bytes we still need to read
                    bytes_to_read -= bytes_available;
                }
                // Refresh the internal buffer with decrypted data
                // TODO block_on considered harmful
                executor::block_on(self_pin.refresh_buffer())?;
            }
        }
        // Get the buffer back from the cursor
        Ok(buf_len - bytes_to_read)
    }
}

// TODO (xBalbinus & thea-exe): Our inline tests
#[cfg(test)]
mod test {
    #[tokio::test]
    /// Test that we can decrypt-read some random piece of data with a random key and nonce without
    /// panicking
    async fn test() {
        use super::DecryptionReader;
        use crate::crypto_tools::key_and_nonce_types::{keygen, KeyAndNonceToDisk};
        use aes_gcm::aes::cipher::crypto_common::rand_core::{OsRng, RngCore};
        use std::io::{Cursor, Read};

        // generate a random key and nonce
        let keygen = keygen();
        // Consume the keygen to get the key and nonce to disk
        let KeyAndNonceToDisk { key, nonce } = keygen.consume_and_prep_to_disk();

        // generate a random piece of data in a 1kb buffer and wrap it in a cursor
        let mut data = vec![0u8; 1024];
        OsRng.fill_bytes(&mut data);
        let data_cursor = Cursor::new(data);

        // Initialize the Decryption Reader
        let mut decryption_reader =
            DecryptionReader::new(data_cursor, KeyAndNonceToDisk { key, nonce })
                .await
                .unwrap();

        // Try and decrypt the data
        let mut decrypted_data = vec![0u8; 1024];
        decryption_reader.read(&mut decrypted_data).unwrap();
        decryption_reader.finish().await.unwrap();

        // If we got here, we didn't panic, so we're good
        return;
    }
}
