use tomb_crypt::prelude::*;
pub(crate) async fn generate_api_key() -> (EcSignatureKey, String) {
    let api_key = EcSignatureKey::generate().await.unwrap();
    let public_api_key = api_key.public_key().unwrap();
    let public_api_key_pem = String::from_utf8(public_api_key.export().await.unwrap()).unwrap();
    (api_key, public_api_key_pem)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) async fn generate_bucket_key() -> (EcEncryptionKey, String) {
    let bucket_key = EcEncryptionKey::generate().await.unwrap();
    let public_bucket_key = bucket_key.public_key().unwrap();
    let public_bucket_key_pem =
        String::from_utf8(public_bucket_key.export().await.unwrap()).unwrap();
    (bucket_key, public_bucket_key_pem)
}
