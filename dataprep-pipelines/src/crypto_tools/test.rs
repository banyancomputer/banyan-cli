#[cfg(test)]
mod tests {
    use crate::crypto_tools::decryption_reader;
    use crate::crypto_tools::encryption_writer;
    use crate::crypto_tools::key_and_nonce_types::KeyAndNonceToDisk;

    #[tokio::test]
    /// Using a Cursor and a 1Kb buffer, test if the encryption_writer and decryption_reader
    /// are able to encrypt and decrypt a file correctly.
    async fn test_encryption_writer_and_decryption_reader_parity() {
        use aes_gcm::aes::cipher::crypto_common::rand_core::{OsRng, RngCore};
        use std::io::{Cursor, Read, Seek, SeekFrom, Write};

        // Create a 1kb buffer with random data to hold our input
        let mut buf_in = [0u8; 1024];
        OsRng.fill_bytes(&mut buf_in);
        // Create a buffer to hold our output
        let mut buf_out = [0u8; 1024];
        // Create a Cursor to hold our encrypted data
        let mut encrypted_data = Cursor::new(Vec::<u8>::new());

        // Encrypt the data ->
        // Initialize the encryption_writer and save the key and nonce
        let (mut enc_writer, KeyAndNonceToDisk { key, nonce }) =
            encryption_writer::EncryptionWriter::new(&mut encrypted_data);
        // Write the data to the encryption_writer
        enc_writer.write(&buf_in).unwrap();
        // Finish the encryption_writer
        enc_writer.finish().await.unwrap();

        // Decrypt the data <-
        // Seek to the beginning of the encrypted data
        encrypted_data.seek(SeekFrom::Start(0)).unwrap();
        // Initialize the decryption_reader
        let mut dec_reader = decryption_reader::DecryptionReader::new(
            encrypted_data,
            KeyAndNonceToDisk { key, nonce },
        )
        .await
        .unwrap();
        // Read the data from the decryption_reader
        dec_reader.read(&mut buf_out).unwrap();
        // Finish the decryption_reader
        dec_reader.finish().await.unwrap();

        // Compare the input and output
        assert_eq!(buf_in, buf_out);
    }
}
