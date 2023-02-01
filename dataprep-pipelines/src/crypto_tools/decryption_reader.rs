use crate::crypto_tools::key_and_nonce_types::{BUF_SIZE, KeyAndNonce, KeyAndNonceToDisk, TAG_SIZE};
use aead::stream::{Decryptor, NewStream, StreamBE32, StreamPrimitive};
use aes_gcm::{Aes256Gcm, KeyInit};
use anyhow::{anyhow, Result};
use futures::executor;
use std::cell::RefCell;
use std::io::{Cursor, ErrorKind, Read, Seek, SeekFrom, Write};
use std::pin::Pin;

/// A wrapper around a writer that encrypts the data as it is written.
/// Should not be used on files larger than 32 GB.
pub struct DecryptionReader<R: Read + Unpin+ Seek> {
    /// Internal buffer, holds data from disk and decrypts it in place
    /// // TODO why a vec? why not a fixed size array?
    buf: RefCell<Vec<u8>>,
    /// tracks how much encrypted data is left in the reader
    bytes_in_reader: RefCell<usize>,
    /// buf_ptr tracks the current position in the buffer- where to start reading decrypted data from
    buf_ptr: RefCell<usize>,
    /// bytes_in_buffer tracks how many bytes are in the buffer- it may not always be full
    bytes_in_buffer: RefCell<usize>,
    /// Writer to another file
    reader: RefCell<R>,
    /// Decryptor. This stores the key
    decryptor: RefCell<Decryptor<Aes256Gcm, StreamBE32<Aes256Gcm>>>,
    /// Counter of bytes read from this struct's read() method
    bytes_read: RefCell<usize>,
    /// buffer to eventually put the tag into
    tag_buf: RefCell<[u8; TAG_SIZE]>,
    /// should we finalize the decryption? this means tag_buf will be filled with the tag and bytes_in_reader will be 0
    finalize: RefCell<bool>,
}

/// A wrapper around a reader that decrypts the data as it is written.
impl<R: Read + Unpin + Seek> DecryptionReader<R> {
    /// Create a new DecryptionReader.
    ///
    /// # Arguments
    /// reader: The reader to read encrypted data from. should start with the nonce.
    /// key_and_nonce: The key and nonce to use for decryption.
    ///
    /// # Returns
    /// A DecryptionReader
    pub async fn new(mut reader: R, key_and_nonce: KeyAndNonceToDisk) -> Result<Self> {
        let KeyAndNonce { key, nonce } = *key_and_nonce.consume_and_prep_from_disk()?;
        let cipher = Aes256Gcm::new(&key);
        let decryptor = RefCell::new(StreamBE32::from_aead(cipher, &nonce).decryptor());

        let file_len = reader.seek(SeekFrom::End(0))?;
        Ok(Self {
            buf: RefCell::new(vec![]),
            tag_buf: RefCell::new([0; TAG_SIZE]),
            bytes_in_reader: RefCell::new(file_len as usize - TAG_SIZE),
            bytes_in_buffer: RefCell::new(0),
            buf_ptr: RefCell::new(0),
            reader: reader.into(),
            decryptor,
            bytes_read: RefCell::new(0),
            finalize: RefCell::new(false),
        })
    }

    /// Read from the Reader into the buffer, decrypting it in place.
    pub async fn refresh_buffer(&mut self) -> std::io::Result<()> {
        println!("Refreshing buffer");
        // Borrow the mutable references to the fields we need
        let mut buf = self.buf.borrow_mut();
        let mut reader = self.reader.borrow_mut();
        let mut decryptor = self.decryptor.borrow_mut();
        let mut buf_ptr = self.buf_ptr.borrow_mut();
        let mut bytes_in_buffer = self.bytes_in_buffer.borrow_mut();
        let mut bytes_in_reader = self.bytes_in_reader.borrow_mut();

        // Assert that our internal buffer is empty
        assert_eq!(*buf_ptr, *bytes_in_buffer);
        // Clear the buffer and reset size
        buf.clear();
        buf.resize(BUF_SIZE, 0);

        // Read from the reader into the buffer
        let new_bytes_read = reader.read(&mut *buf)?;
        println!("Read {} bytes", new_bytes_read);

        if new_bytes_read >= *bytes_in_reader {
            // let's get the tag.
            // there may be some tag bytes in the buffer, but maybe not all of them.
            // we need to read the rest of any of them from the reader.
            let mut tag_buf = self.tag_buf.borrow_mut();
            // get the tag bytes out of the buffer
            tag_buf[..new_bytes_read - *bytes_in_reader].copy_from_slice(&buf[*bytes_in_reader..new_bytes_read]);
            // zero that part of the buffer
            buf[*bytes_in_reader..new_bytes_read].fill(0);
            // read the rest of the tag from the reader
            reader.read_exact(&mut tag_buf[new_bytes_read - *bytes_in_reader..])?;
            // set buf_ptr
            *buf_ptr = 0;
            // set bytes_in_buffer
            *bytes_in_buffer = *bytes_in_reader;
            // in this case, the last 16 bytes we read are the tag
            *bytes_in_reader = 0;
            // set the finalize flag
            *self.finalize.borrow_mut() = true;
        } else {
            // Otherwise, there's more data to decrypt
            *bytes_in_reader -= new_bytes_read;
            // set buf_ptr
            *buf_ptr = 0;
            // set bytes_in_buffer
            *bytes_in_buffer = new_bytes_read;
        }
        // if we got anything, decrypt it
        if *bytes_in_buffer > 0 {
            // TODO laudiacay i think this copies
            let mut buf_to_decrypt=  buf[..*bytes_in_buffer].to_vec();
            // Decrypt the buffer in place
            (*decryptor)
                .decrypt_next_in_place(b"".as_ref(), &mut buf_to_decrypt)
                .map_err(|_| {
                    std::io::Error::new(ErrorKind::Other, anyhow!("Error decrypting block!"))
                })?;
        };
        Ok(())
    }

    pub async fn finish(self) -> Result<usize> {
        assert_eq!(self.bytes_in_reader.borrow().clone(), 0);
        assert_eq!(self.finalize.borrow().clone(), true);
        assert_eq!(self.bytes_in_buffer.borrow().clone(), self.buf_ptr.borrow().clone());
        // TODO maybe add a sanity check for bytes_read!
        self.decryptor
            .into_inner()
            .decrypt_last_in_place(b"".as_ref(), &mut self.tag_buf.borrow().to_vec())
            .map_err(|_| anyhow!("Error decrypting last block"))?;
        Ok(*self.bytes_read.borrow_mut())
    }

    pub fn cipher_info(&self) -> String {
        "AES-256-GCM".to_string()
    }
}

/// Implement the Read trait for DecryptionReader.
impl<R: Read + Unpin + Seek> Read for DecryptionReader<R> {
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
        println!("made it");
        while bytes_to_read > 0 && !self_pin.finalize.borrow().clone() {
            println!("made it 2");
            // There's still bytes to read, and we're not at the end of the file

            // Determine how many bytes are available in the internal buffer
            let bytes_available = *self_pin.bytes_in_buffer.borrow() - *self_pin.buf_ptr.borrow();
            // If there's more to read than there's space available, read the space available
            println!("bytes available: {}", bytes_available);
            if bytes_available >= bytes_to_read {
                {
                    println!("we should not be here");
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
                println!("we should be here");
                // Do we need to write if bytes_available == 0?
                {
                    let mut buf_ptr = self_pin.buf_ptr.borrow_mut();
                    // Copy all available bytes from the internal buffer into the output buffer
                    buf_cursor
                        .write_all(&self_pin.buf.borrow()[*buf_ptr..*buf_ptr + bytes_available])?;
                    // Update the buffer pointer (it should now be at the end of the buffer, == bytes_in_buffer)
                    *buf_ptr += bytes_available;
                    // Update the number of bytes we still need to read
                    bytes_to_read -= bytes_available;
                    println!("state check: {}", bytes_to_read);
                    println!("state check: {}", bytes_available);
                    println!("state check: {}", *buf_ptr);
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
    use aead::Payload;
    use aead::stream::{NewStream, StreamBE32, StreamPrimitive};
    use aes_gcm::KeyInit;
    use crate::crypto_tools::key_and_nonce_types::KeyAndNonce;

    #[tokio::test]
    /// Test that we can decrypt-read some random piece of data with a random key and nonce without
    /// panicking
    async fn test() {
        use super::DecryptionReader;
        use crate::crypto_tools::key_and_nonce_types::{keygen, KeyAndNonceToDisk};
        use aes_gcm::aes::cipher::crypto_common::rand_core::{OsRng, RngCore};
        use std::io::{Cursor, Read};

        // generate a random key and nonce
        let keygen @ KeyAndNonce {key, nonce} = keygen();

        // generate an all-zero piece of data in a 1kb buffer and wrap it in a cursor
        let mut data = vec![0u8; 1024];
        // encrypt the data
        let mut enc = StreamBE32::from_aead(aes_gcm::Aes256Gcm::new(&key), &nonce).encryptor();
        let encrypted = enc.encrypt_next(Payload::from(&*data)).unwrap();

        // Initialize the Decryption Reader
        let mut decryption_reader =
            DecryptionReader::new(Cursor::new(encrypted),  keygen.consume_and_prep_to_disk())
                .await
                .unwrap();

        // Try and decrypt the data
        let mut decrypted_data = vec![1u8; 1024];
        decryption_reader.read(&mut decrypted_data).unwrap();
        decryption_reader.finish().await.unwrap();

        // check that decrypted_data is now all zeros
        assert_eq!(decrypted_data, vec![0u8; 1024]);
    }
}
