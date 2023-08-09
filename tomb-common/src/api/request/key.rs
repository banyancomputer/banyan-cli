use std::{convert::Infallible, fmt::Display};

use clap::{Args, Subcommand};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::api::error::StatusError;

use super::Requestable;

const API_PREFIX: &str = "/api/v1/auth";

#[derive(Debug, Clone, Serialize, Subcommand)]
pub enum KeyRequest {
    Create(CreateKeyRequest),
    Get(GetKeyRequest),
    Delete(DeleteKeyRequest),
}

#[derive(Debug, Clone, Serialize, Args)]
pub struct CreateKeyRequest;
#[derive(Debug, Clone, Serialize, Args)]
pub struct GetKeyRequest {
    fingerprint: String,
}
#[derive(Debug, Clone, Serialize, Args)]
pub struct DeleteKeyRequest;

impl Requestable for CreateKeyRequest {
    type ErrorType = StatusError;
    type ResponseType = CreateKeyResponse;

    fn endpoint(&self) -> String {
        format!("{}", API_PREFIX)
    }
    fn method(&self) -> Method {
        Method::POST
    }
    fn authed(&self) -> bool {
        true
    }
}

impl Requestable for GetKeyRequest {
    type ErrorType = StatusError;
    type ResponseType = GetKeyResponse;

    fn endpoint(&self) -> String {
        format!("{}", API_PREFIX)
    }
    fn method(&self) -> Method {
        Method::GET
    }
    fn authed(&self) -> bool {
        true
    }
}

impl Requestable for DeleteKeyRequest {
    type ErrorType = StatusError;
    type ResponseType = DeleteKeyResponse;

    fn endpoint(&self) -> String {
        format!("{}", API_PREFIX)
    }
    fn method(&self) -> Method {
        Method::DELETE
    }
    fn authed(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateKeyResponse {
    /// Public Key PEM string
    public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct GetKeyResponse {
    /// Public Key PEM string
    public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteKeyResponse;

#[cfg(test)]
mod test {
    use crate::api::{
        error::ClientError,
        request::{fake::*, CreateKeyRequest, DeleteKeyRequest, GetKeyRequest},
    };
    use serial_test::serial;
    use tomb_crypt::{
        prelude::{EcPublicEncryptionKey, WrappingPublicKey},
        pretty_fingerprint,
    };

    #[tokio::test]
    #[serial]
    async fn create() -> Result<(), ClientError> {
        let (mut client, _, public_key) = setup().await?;
        let public_pem = String::from_utf8(public_key.export().await.unwrap()).unwrap();
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
