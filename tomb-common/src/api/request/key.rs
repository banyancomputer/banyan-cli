use super::{RequestMetadata, Requestable};
use crate::api::error::StatusError;
use clap::{Args, Subcommand};
use reqwest::Method;
use serde::{Deserialize, Serialize};

const API_PREFIX: &str = "/api/v1/auth";

/// Key requests
#[derive(Debug, Clone, Serialize, Subcommand)]
pub enum KeyRequest {
    /// Create a Key
    Create(CreateKeyRequest),
    /// Get a Key
    Get(GetKeyRequest),
    /// Delete a Key
    Delete(DeleteKeyRequest),
}

/// Request to create a Key
#[derive(Debug, Clone, Serialize, Args)]
pub struct CreateKeyRequest;

/// Request to get a Key
#[derive(Debug, Clone, Serialize, Args)]
pub struct GetKeyRequest {
    fingerprint: String,
}

/// Request to delete a Key
#[derive(Debug, Clone, Serialize, Args)]
pub struct DeleteKeyRequest;

impl Requestable for CreateKeyRequest {
    type ErrorType = StatusError;
    type ResponseType = CreateKeyResponse;

    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: API_PREFIX.to_string(),
            method: Method::POST,
            auth: true,
        }
    }
}

impl Requestable for GetKeyRequest {
    type ErrorType = StatusError;
    type ResponseType = GetKeyResponse;

    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: API_PREFIX.to_string(),
            method: Method::GET,
            auth: true,
        }
    }
}

impl Requestable for DeleteKeyRequest {
    type ErrorType = StatusError;
    type ResponseType = DeleteKeyResponse;

    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: API_PREFIX.to_string(),
            method: Method::DELETE,
            auth: true,
        }
    }
}

/// Response from requesting to create a Key
#[derive(Debug, Deserialize)]
pub struct CreateKeyResponse {
    /// Public Key PEM string
    #[allow(dead_code)]
    public_key: String,
}

/// Response from requesting to get a Key
#[derive(Debug, Deserialize)]
pub struct GetKeyResponse {
    /// Public Key PEM string
    #[allow(dead_code)]
    public_key: String,
}

/// Response from requesting to delete a Key
#[derive(Debug, Deserialize)]
pub struct DeleteKeyResponse;

#[cfg(test)]
mod test {
    use crate::api::{
        error::ClientError,
        request::{fake::*, CreateKeyRequest, DeleteKeyRequest},
    };
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn create() -> Result<(), ClientError> {
        let (mut client, _, _) = setup().await?;

        let request = CreateKeyRequest;
        let response = client.send(request).await;

        assert!(response.is_err());

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn create_get() -> Result<(), ClientError> {
        let (mut client, _, _) = setup().await?;

        let request = CreateKeyRequest;
        let response1 = client.send(request).await;

        assert!(response1.is_err());

        // let public_key = EcPublicEncryptionKey::import(response1.public_key.as_bytes()).await.expect("unable to import key");
        // let fingerprint = pretty_fingerprint(&public_key.fingerprint().await.expect("unable to make fingerprint"));

        // let _response2 = client
        //     .send(GetKeyRequest { fingerprint })
        //     .await?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn delete() -> Result<(), ClientError> {
        let (mut client, _, _) = setup().await?;
        let response = client.send(DeleteKeyRequest).await;

        assert!(response.is_err());

        Ok(())
    }
}
