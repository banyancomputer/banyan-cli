// // THIS FILE IS DEPRECATED- WE ARE USING AGE INSTEAD
//
//
// use crate::crypto_tools::key_and_nonce_types::{keygen, KeyAndNonce, KeyAndNonceToDisk, TAG_SIZE, BUF_SIZE};
// use aead::stream::NewStream;
// use aead::stream::{Encryptor, StreamBE32, StreamPrimitive};
//
// use aes_gcm::{Aes256Gcm, KeyInit};
// use anyhow::Result;
//
// use futures::executor;
//
// use std::cell::RefCell;
// use std::io::prelude::Write;
// use std::pin::Pin;
// use aead::Payload;
// use tokio::io::{AsyncSeek, AsyncWrite, AsyncWriteExt};
//
// /// A wrapper around a writer that encrypts the data as it is written.
// /// Should not be used on files larger than 32 GB.
// pub struct EncryptionWriter<W: AsyncWrite + Unpin> {
//     /// Internal buffer, holds data to be encrypted in place
//     buf: RefCell<Vec<u8>>,
//     /// Pointer to the end of the buffer
//     buf_ptr: RefCell<usize>,
//     /// Writer to another file
//     writer: RefCell<W>,
//     /// Encryptor. This stores the key
//     encryptor: RefCell<Encryptor<Aes256Gcm, StreamBE32<Aes256Gcm>>>,
//     /// Counter of bytes written
//     bytes_written: RefCell<usize>,
//     /// size limit for buffer
//     size_limit: usize,
// }
//
// /// A wrapper around a writer that encrypts the data as it is written.
// impl<W: AsyncWrite + Unpin> EncryptionWriter<W> {
//     /// Create a new EncryptionWriter.
//     ///
//     /// # Arguments
//     /// writer: The writer to write encrypted data to.
//     /// key: The key to use for encryption.
//
//     pub fn new(writer: W) -> (Self, KeyAndNonceToDisk) {
//         // keygen
//         let keygen @ KeyAndNonce { key, nonce } = keygen();
//
//         // write space for the tag to the start of the file
//         // TODO (laudiacay): this is a hack, fix it
//         let mut writer = writer;
//
//         // Create the encryptor.
//         let cipher = Aes256Gcm::new(&key);
//         let encryptor = RefCell::new(StreamBE32::from_aead(cipher, &nonce).encryptor());
//
//         // get the file length
//
//         (
//             Self {
//                 buf: RefCell::new(vec![0; BUF_SIZE + TAG_SIZE]),
//                 buf_ptr: RefCell::new(0),
//                 writer: writer.into(),
//                 encryptor,
//                 size_limit: BUF_SIZE, // TODO (laudiacay) maybe one day make changeable
//                 bytes_written: RefCell::new(0),
//             },
//             keygen.consume_and_prep_to_disk(),
//         )
//     }
//
//     /// Encrypt the data in the buffer and write it to the writer.
//     pub async fn finish(mut self) -> Result<usize> {
//         self.flush()?;
//         // TODO (laudiacay): check this logic better, especially once your PR is merged on rust crypto
//         self.encryptor
//             .into_inner()
//             .encrypt_last(Payload::from(b"finalize"))
//             .unwrap();
//         executor::block_on(self.writer.borrow_mut().write_all(&self.buf.borrow_mut()))?;
//         *self.bytes_written.borrow_mut() += self.buf.borrow().len();
//         Ok(*self.bytes_written.borrow())
//     }
//
//     pub fn cipher_info(&self) -> String {
//         "AES-256-GCM".to_string()
//     }
// }
//
// /// Implement the Write trait for EncryptionWriter.
// impl<W: AsyncWrite + Unpin > Write for EncryptionWriter<W> {
//     /// Write data to the buffer
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         let mut self_pin = Pin::new(&mut *self);
//         // how long is buf?
//         // if it's too long, we need to split it up so we're not encrypting more than the buffer size at a time
//         let mut buf = buf;
//         while !buf.is_empty() {
//             // get the buffer pointer
//             let mut buf_ptr = self_pin.buf_ptr.borrow_mut();
//             // how much of the buf can we fit in the buffer?
//             if *buf_ptr + buf.len() <= self_pin.size_limit {
//                 // if we can fit it all in the buffer, do that
//                 self_pin.buf.borrow_mut()[*buf_ptr..*buf_ptr + buf.len()].copy_from_slice(buf);
//                 *buf_ptr += buf.len();
//                 let buf = &buf[buf.len()..];
//             } else {
//                 // grab what we can fit in the buffer
//                 let (buf1, buf2) = buf.split_at(self_pin.size_limit - *buf_ptr);
//                 // stick it in there
//                 self_pin.buf.borrow_mut()[*buf_ptr..*buf_ptr + buf1.len()].copy_from_slice(buf1);
//                 // flush if we're full
//                 self_pin.flush()?;
//                 // set the buf to the rest of the data
//                 buf = buf2;
//             }
//         }
//         Ok(buf.len())
//     }
//
//     /// Clear the buffer and encrypt the data in place.
//     fn flush(&mut self) -> std::io::Result<()> {
//         let self_pin = Pin::new(&mut *self);
//
//         assert!(*self.buf_ptr.borrow() == self.size_limit);
//         // do the encryption
//         self_pin
//             .encryptor
//             .borrow_mut()
//             .encrypt_next_in_place(b"", &mut *self_pin.buf.borrow_mut())
//             .unwrap();
//
//         // TODO (laudiacay): YIKES! is this what we want? block_on???
//         // write encrypted data to underlying writer
//         executor::block_on(
//             self_pin
//                 .writer
//                 .borrow_mut()
//                 .write_all(&self_pin.buf.borrow()),
//         )?;
//
//         // TODO (laudiacay) is this right to put here? probably... but make sure :)
//         // flush underlying writer to wherever it's going i guess
//         executor::block_on(self_pin.writer.borrow_mut().flush())?;
//
//         // update counter for how many bytes you wrote
//         *self_pin.bytes_written.borrow_mut() += self_pin.buf.borrow().len();
//
//         // clear out the buffer
//         self_pin.buf.borrow_mut().clear();
//         Ok(())
//     }
// }
//
// // TODO (xBalbinus & thea-exe): Our inline tests
// #[cfg(test)]
// mod test {
//     #[tokio::test]
//     /// Test that we can encrypt write some data to a cursor without panicking.
//     async fn test() {
//         use super::EncryptionWriter;
//         use aes_gcm::aes::cipher::crypto_common::rand_core::{OsRng, RngCore};
//         use std::io::{Cursor, Write};
//
//         // generate a random piece of data in a 1kb buffer
//         let mut data = vec![0u8; 1024];
//         OsRng.fill_bytes(&mut data);
//         // Declare a new cursor to write to
//         let mut cursor = Cursor::new(Vec::<u8>::new());
//
//         // Create a new EncryptionWriter
//         let (mut encryptor, _) = EncryptionWriter::new(&mut cursor);
//         // Try Encrypting the data to the cursor
//         encryptor.write(&data).unwrap();
//         // Finish the encryption
//         encryptor.finish().await.unwrap();
//         // If we got here, we didn't panic, so we're good
//         return;
//     }
// }
