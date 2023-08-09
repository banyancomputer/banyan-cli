use super::{Requestable, RequestMetadata};
use crate::api::{
    client::Client,
    credentials::Credentials,
    error::{ClientError, InfallibleError},
    request::WhoRequest,
};
use jsonwebtoken::{get_current_timestamp, EncodingKey};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use tomb_crypt::{
    prelude::{EcEncryptionKey, EcPublicEncryptionKey, WrappingPrivateKey, WrappingPublicKey},
    pretty_fingerprint,
};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterAccountRequest;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterDeviceKeyRequest {
    pub token: String,
    pub public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterAccountResponse {
    pub id: Uuid,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceKeyResponse {
    pub id: Uuid,
    pub account_id: Uuid,
    pub fingerprint: String,
}

impl Requestable for RegisterAccountRequest {
    type ResponseType = RegisterAccountResponse;
    type ErrorType = InfallibleError;

    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: format!("/api/v1/auth/create_fake_account"),
            method: Method::GET,
            auth: false,
        }
    }
}

impl Requestable for RegisterDeviceKeyRequest {
    type ResponseType = RegisterDeviceKeyResponse;
    type ErrorType = InfallibleError;
    
    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: format!("/api/v1/auth/fake_register_device_key"),
            method: Method::POST,
            auth: false,
        }
    }
}

pub async fn setup() -> Result<(Client, EcEncryptionKey, EcPublicEncryptionKey), ClientError> {
    const TEST_REMOTE: &str = "http://127.0.0.1:3001/";
    let mut client = Client::new(TEST_REMOTE).unwrap();
    // Register
    let account_response = client.send(RegisterAccountRequest).await?;
    // Create a local key pair
    let private_key = EcEncryptionKey::generate().await.unwrap();
    let public_key = private_key.public_key().unwrap();
    // Represent as PEM string
    let public_key_pem_string = String::from_utf8(public_key.export().await.unwrap()).unwrap();
    // Set the bearer token
    client.bearer_token = Some((
        get_current_timestamp() + 870,
        account_response.token.clone(),
    ));
    // Assert that it's accessible
    assert!(client.bearer_token().is_some());

    //
    let jwt_signing_key = EncodingKey::from_ec_pem(&private_key.export().await.unwrap()).unwrap();

    // Update the credentials
    client.credentials = Some(Credentials {
        account_id: account_response.id,
        fingerprint: pretty_fingerprint(&public_key.fingerprint().await.unwrap()),
        signing_key: jwt_signing_key,
    });

    // Send a device registration request
    let device_response = client
        .send(RegisterDeviceKeyRequest {
            token: account_response.token,
            public_key: public_key_pem_string,
        })
        .await?;

    // Empty out the broken bearer token
    client.bearer_token = None;

    let authenticated_info = client.send(WhoRequest).await?;
    assert_eq!(authenticated_info.account_id, device_response.account_id);

    Ok((client, private_key, public_key))
}
