use openssl::pkey::{PKey, Private, Public};

mod error;

#[cfg(not(target = "wasm"))]
mod internal;
#[cfg(target = "wasm")]
mod wasm_internal;

pub use error::KeySealError;

const AES_KEY_SIZE: usize = 32;
const ECDH_SECRET_BYTE_SIZE: usize = 48;
const FINGERPRINT_SIZE: usize = 20;
const SALT_SIZE: usize = 16;

pub fn pretty_fingerprint(fingerprint_bytes: &[u8]) -> String {
    fingerprint_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<String>>()
        .join(":")
}

pub struct EcEncryptionKey(PKey<Private>);

impl EcEncryptionKey {
    pub fn export(&self) -> Result<Vec<u8>, KeySealError> {
        self.0
            .private_key_to_pem_pkcs8()
            .map_err(KeySealError::export_failed)
    }

    pub fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        self.0
            .private_key_to_der()
            .map_err(KeySealError::export_failed)
    }

    pub fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        self.public_key()?.fingerprint()
    }

    pub fn generate() -> Result<Self, KeySealError> {
        Ok(Self(internal::generate_ec_key()))
    }

    pub fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        let raw_private =
            PKey::private_key_from_pkcs8(pem_bytes).map_err(KeySealError::bad_format)?;

        Ok(Self(raw_private))
    }

    pub fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let raw_private =
            PKey::private_key_from_der(der_bytes).expect("parsing a valid der private key");
        Ok(Self(raw_private))
    }

    pub fn public_key(&self) -> Result<EcPublicEncryptionKey, KeySealError> {
        let ec_public = internal::public_from_private(&self.0);
        Ok(EcPublicEncryptionKey(ec_public))
    }
}

pub struct EcPublicEncryptionKey(PKey<Public>);

impl EcPublicEncryptionKey {
    pub fn export(&self) -> Result<Vec<u8>, KeySealError> {
        self.0
            .public_key_to_pem()
            .map_err(KeySealError::export_failed)
    }

    pub fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        self.0
            .public_key_to_der()
            .map_err(KeySealError::export_failed)
    }

    pub fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        Ok(internal::fingerprint(&self.0))
    }

    pub fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        let raw_public =
            PKey::public_key_from_pem(pem_bytes).expect("parsing a valid pem public key");
        Ok(Self(raw_public))
    }

    pub fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let raw_public =
            PKey::public_key_from_der(der_bytes).expect("parsing a valid der public key");
        Ok(Self(raw_public))
    }
}

pub struct EncryptedSymmetricKey {
    data: [u8; AES_KEY_SIZE + 8],
    salt: [u8; SALT_SIZE],
    public_key: Vec<u8>,
}

impl EncryptedSymmetricKey {
    pub fn decrypt_with(
        &self,
        recipient_key: &EcEncryptionKey,
    ) -> Result<SymmetricKey, KeySealError> {
        let ephemeral_public_key = EcPublicEncryptionKey::import_bytes(self.public_key.as_ref())?;
        let ecdh_shared_secret = internal::ecdh_exchange(&recipient_key.0, &ephemeral_public_key.0);

        let info = internal::generate_info(
            ephemeral_public_key.fingerprint()?.as_ref(),
            recipient_key.fingerprint()?.as_ref(),
        );
        let hkdf_shared_secret =
            internal::hkdf_with_salt(&ecdh_shared_secret, self.salt.as_ref(), &info);

        let temporal_key_bytes = internal::unwrap_key(&hkdf_shared_secret, self.data.as_ref());

        Ok(SymmetricKey(temporal_key_bytes))
    }

    pub fn export(&self) -> String {
        [
            internal::base64_encode(&self.salt),
            internal::base64_encode(&self.data),
            internal::base64_encode(self.public_key.as_ref()),
        ]
        .join(".")
    }

    pub fn import(serialized: &str) -> Result<Self, KeySealError> {
        let components: Vec<_> = serialized.split('.').collect();

        let raw_salt = internal::base64_decode(components[0])?;
        let mut salt = [0u8; SALT_SIZE];
        salt.copy_from_slice(raw_salt.as_ref());

        let raw_data = internal::base64_decode(components[1])?;
        let mut data = [0u8; AES_KEY_SIZE + 8];
        data.copy_from_slice(raw_data.as_ref());

        let public_key = internal::base64_decode(components[2])?;

        Ok(Self {
            salt,
            data,
            public_key,
        })
    }
}

pub struct SymmetricKey([u8; AES_KEY_SIZE]);

impl SymmetricKey {
    pub fn encrypt_for(
        &self,
        recipient_key: &EcPublicEncryptionKey,
    ) -> Result<EncryptedSymmetricKey, KeySealError> {
        let ephemeral_key = EcEncryptionKey::generate()?;

        let ecdh_shared_secret = internal::ecdh_exchange(&ephemeral_key.0, &recipient_key.0);

        let info = internal::generate_info(
            ephemeral_key.fingerprint()?.as_ref(),
            recipient_key.fingerprint()?.as_ref(),
        );
        let (salt, hkdf_shared_secret) = internal::hkdf(&ecdh_shared_secret, &info);

        let encrypted_key = internal::wrap_key(&hkdf_shared_secret, &self.0);
        let exported_ephemeral_key = ephemeral_key.public_key()?.export_bytes()?;

        Ok(EncryptedSymmetricKey {
            data: encrypted_key,
            salt,
            public_key: exported_ephemeral_key,
        })
    }

    #[cfg(test)]
    fn generate() -> Self {
        let mut key_data = [0u8; AES_KEY_SIZE];
        openssl::rand::rand_bytes(&mut key_data).expect("unable to generate key data");
        Self(key_data)
    }
}

impl AsRef<[u8]> for SymmetricKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; AES_KEY_SIZE]> for SymmetricKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}
