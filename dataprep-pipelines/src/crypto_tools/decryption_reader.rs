use crate::crypto_tools::key_and_nonce_types::{KeyAndNonce, KeyAndNonceToDisk};
use aead::stream::NewStream;
use aead::stream::{Decryptor, StreamBE32, StreamPrimitive};
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

/// A wrapper around a writer that decrypts the data as it is written.
impl<R: Read + Unpin> DecryptionReader<R> {
    /// Create a new DecryptionReader.
    ///
    /// # Arguments
    /// reader: The reader to read encrypted data from. should start with the nonce.
    /// key: The key to use for decryption.

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

    pub async fn refresh_buffer(&mut self) -> std::io::Result<()> {
        let mut buf = self.buf.borrow_mut();
        let mut reader = self.reader.borrow_mut();
        let mut decryptor = self.decryptor.borrow_mut();
        let mut buf_ptr = self.buf_ptr.borrow_mut();
        let mut bytes_in_buffer = self.bytes_in_buffer.borrow_mut();
        let mut eof = self.eof.borrow_mut();
        // ensure we're out of data!
        assert_eq!(*buf_ptr, *bytes_in_buffer);
        // clear the buffer
        buf.clear();
        // fill it up
        buf.resize(BUF_SIZE, 0);
        let new_bytes_read = reader.read(&mut buf)?;
        // are we at the end of the file?
        if new_bytes_read == 0 {
            *eof = true;
        } else {
            (*decryptor)
                .decrypt_next_in_place(b"".as_ref(), &mut *buf)
                .map_err(|_| {
                    std::io::Error::new(ErrorKind::Other, anyhow!("Error decrypting block!"))
                })?;
        };
        // update buffer info
        *bytes_in_buffer = new_bytes_read;
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
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes_to_read = buf.len();
        // put a cursor on the buffer
        let mut buf_cursor = Cursor::new(buf);
        let mut self_pin = Pin::new(&mut *self);
        // read as many bytes as we need into the internal buffer to fill this request
        while bytes_to_read > 0 && !*self_pin.eof.borrow() {
            let bytes_available = *self_pin.bytes_in_buffer.borrow() - *self_pin.buf_ptr.borrow();
            // if we have enough bytes in the buffer to fill this request, do it
            if bytes_available >= bytes_to_read {
                {
                    let buf_ptr = self_pin.buf_ptr.borrow();
                    // copy the bytes from the internal buffer into the output buffer
                    buf_cursor
                        .write_all(&self_pin.buf.borrow()[*buf_ptr..*buf_ptr + bytes_to_read])?;
                    // update the buffer pointer
                    *self_pin.buf_ptr.borrow_mut() += bytes_to_read;
                    // we're done
                    bytes_to_read = 0;
                }
            } else {
                {
                    let mut buf_ptr = self_pin.buf_ptr.borrow_mut();
                    // copy the bytes from the internal buffer into the output buffer
                    buf_cursor
                        .write_all(&self_pin.buf.borrow()[*buf_ptr..*buf_ptr + bytes_available])?;
                    // update the buffer pointer
                    *buf_ptr += bytes_available;
                    // update the number of bytes we still need to read
                    bytes_to_read -= bytes_available;
                }
                // refresh the buffer
                // TODO block_on considered harmful
                executor::block_on(self_pin.refresh_buffer())?;
            }
        }
        let buf = buf_cursor.into_inner();
        Ok(buf.len() - bytes_to_read)
    }
}

// TODO (xBalbinus & thea-exe): Our inline tests
#[cfg(test)]
mod test {
    #[tokio::test]
    /// Test that we can decrypt-read some random piece of data with a random key and nonce without
    /// panicking
    async fn test() {
        use crate::crypto_tools::key_and_nonce_types::{keygen, KeyAndNonceToDisk};
        use super::DecryptionReader;
        use aes_gcm::aes::cipher::crypto_common::rand_core::{OsRng, RngCore};
        use std::io::{Cursor, Read};

        // generate a random key and nonce
        let keygen = keygen();
        // Consume the keygen to get the key and nonce to disk
        let KeyAndNonceToDisk { key, nonce } = keygen.consume_and_prep_to_disk();

        // generate a random piece of data in a 1kb buffer and wrap it in a cursor
        let mut data = vec![0u8; 1024];
        OsRng.fill_bytes(&mut data);
        let mut data_cursor = Cursor::new(data);

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
