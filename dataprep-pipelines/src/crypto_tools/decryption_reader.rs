use crate::crypto_tools::key_and_nonce_types::{KeyAndNonce, KeyAndNonceToDisk};
use aead::stream::NewStream;
use aead::stream::{Decryptor, StreamBE32, StreamPrimitive};
use aes_gcm::{Aes256Gcm, KeyInit};
use anyhow::Result;
use std::cell::RefCell;
use std::io::Read;

const BUF_SIZE: usize = 1024 * 1024; // 1 MB

/// A wrapper around a writer that encrypts the data as it is written.
/// Should not be used on files larger than 32 GB.
pub struct DecryptionReader<R: Read + Unpin> {
    /// Internal buffer, holds data to be encrypted in place
    buf: RefCell<Vec<u8>>,
    /// Writer to another file
    reader: RefCell<R>,
    /// Decryptor. This stores the key
    decryptor: RefCell<Decryptor<Aes256Gcm, StreamBE32<Aes256Gcm>>>,
    /// Counter of bytes read
    bytes_read: RefCell<usize>,
    /// size limit for buffer
    size_limit: usize,
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
            reader: reader.into(),
            decryptor,
            size_limit: BUF_SIZE, // TODO (laudiacay) maybe one day make changeable
            bytes_read: RefCell::new(0),
        })
    }

    // /// decrypt and output whatever's in the buffer, check the tag, go home
    // // TODO definitely wrong?
    // pub async fn finish(mut self) -> Result<usize> {
    //     // TODO check that you've read the reader all the way thru
    //     // TODO (laudiacay): check this logic better, especially once your PR is merged on rust crypto
    //     self.decryptor
    //         .into_inner()
    //         .decrypt_last_in_place(b"".as_ref(), &mut *self.buf.borrow_mut())?;
    //     *self.bytes_read.borrow_mut() += self.buf.borrow().len();
    //     Ok(*self.read.borrow())
    // }

    pub fn cipher_info(&self) -> String {
        "AES-256-GCM".to_string()
    }
}

/// Implement the Read trait for DecryptionReader.
impl<R: Read + Unpin> Read for DecryptionReader<R> {
    // /// Write data to the buffer
    // fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    //     let mut self_pin = Pin::new(&mut *self);
    //     // how long is buf?
    //     // if it's too long, we need to split it up so we're not encrypting more than the buffer size at a time
    //     let mut buf = buf;
    //     while !buf.is_empty() {
    //         // figure out how much space is left
    //         let remaining_space = self_pin.size_limit - self_pin.buf.borrow().len();
    //
    //         // grab what we can fit in the buffer
    //         let (buf1, buf2) = buf.split_at(remaining_space);
    //
    //         // stick it in there
    //         self_pin.buf.borrow_mut().extend_from_slice(buf1);
    //
    //         // flush if we're full
    //         if self_pin.buf.borrow().len() >= self_pin.size_limit {
    //             self_pin.flush()?;
    //         };
    //
    //         // advance the buffer
    //         buf = buf2;
    //     }
    //     Ok(buf.len())
    // }
    //
    // /// Clear the buffer and encrypt the data in place.
    // fn flush(&mut self) -> std::io::Result<()> {
    //     let self_pin = Pin::new(&mut *self);
    //
    //     // do the encryption
    //     self_pin
    //         .encryptor
    //         .borrow_mut()
    //         .encrypt_next_in_place(b"", &mut *self_pin.buf.borrow_mut())
    //         .unwrap();
    //
    //     // TODO (laudiacay): YIKES! is this what we want? block_on???
    //     // write encrypted data to underlying writer
    //     executor::block_on(
    //         self_pin
    //             .writer
    //             .borrow_mut()
    //             .write_all(&self_pin.buf.borrow()),
    //     )?;
    //
    //     // TODO (laudiacay) is this right to put here? probably... but make sure :)
    //     // flush underlying writer to wherever it's going i guess
    //     executor::block_on(self_pin.writer.borrow_mut().flush())?;
    //
    //     // update counter for how many bytes you wrote, check for safe GCM usage limits
    //     *self_pin.bytes_written.borrow_mut() += self_pin.buf.borrow().len();
    //     if *self.bytes_written.borrow() >= MAX_SAFE_ENCRYPTION_SIZE {
    //         return Err(std::io::Error::new(
    //             std::io::ErrorKind::FileTooLarge,
    //             "File too large to encrypt",
    //         ));
    //     };
    //
    //     // clear out the buffer
    //     self.buf.borrow_mut().clear();
    //     Ok(())
    // }

    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
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
