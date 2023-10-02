use crate::banyan_api::{
    client::{Client, Credentials},
    error::ClientError,
    requests::core::{
        auth::{
            device_api_key::regwait::{Regwait, RegwaitResponse},
            fake_account::create::*,
            who_am_i::read::*,
        },
        buckets::usage::{GetTotalUsage, GetUsageLimit},
    },
    utils::generate_api_key,
};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tomb_crypt::prelude::{EcSignatureKey, PrivateKey, PublicKey};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
/// Account Definition
pub struct Account {
    /// The unique identifier for the account
    pub id: uuid::Uuid,
}

impl Account {
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    pub async fn create_fake(client: &mut Client) -> Result<(Self, EcSignatureKey), ClientError> {
        // Create a local key pair for signing
        let (api_key, device_api_key_pem) = generate_api_key().await;
        // Associate the key material with the backend
        let response: CreateAccountResponse = client
            .call(CreateFakeAccount { device_api_key_pem })
            .await?;

        // Associate the returned account ID with the key material and initialize the client with these credentials
        client.with_credentials(Credentials {
            account_id: response.id,
            signing_key: api_key.clone(),
        });
        // Return the account
        Ok((Self { id: response.id }, api_key))
    }

    /// Log in to an existing account
    pub async fn register_device(
        client: Client,
        private_device_key: EcSignatureKey,
    ) -> Result<Self, ClientError> {
        // Create a public key from the
        let public_device_key = private_device_key
            .public_key()
            .map_err(ClientError::crypto_error)?;
        // Bytes of the public device key
        let public_device_key_bytes = public_device_key
            .export()
            .await
            .map_err(ClientError::crypto_error)?;
        // Public device key in PEM format
        let public_device_key =
            String::from_utf8(public_device_key_bytes).expect("cant convert key bytes to string");
        // Strip the public key of its new lines
        let mut stripped_public_key = public_device_key.replace('\n', "");
        // Strip the public key of its prefix and suffix
        stripped_public_key = stripped_public_key
            .strip_prefix("-----BEGIN PUBLIC KEY-----")
            .expect("unable to strip PEM prefix")
            .strip_suffix("-----END PUBLIC KEY-----")
            .expect("unable to strip PEM suffix")
            .to_string();

        // Represent the weird b64 characters with ones that are url-valid
        let encoded_public_key = stripped_public_key
            .replace('+', "-")
            .replace('/', "_")
            .replace('=', ".")
            .to_string();

        // Create a new nonce to identify this registration
        let nonce = Uuid::new_v4();
        // Create a new request object with the nonce
        let start_regwait = Regwait { nonce };
        // Create a base64 url encoded version of the nonce
        let b64_nonce = base64_url::encode(&nonce);
        // Create a clone of the client to move into the handle
        let mut client_1 = client.clone();
        // Create a join handle for later use, starting the call immediately
        let join_handle: JoinHandle<Result<RegwaitResponse, ClientError>> =
            tokio::task::spawn(async move {
                // Build the request
                client_1.call(start_regwait).await
            });

        // Base url for the frontend
        let base_url = "http://127.0.0.1:3000";
        // Should be this in prod TODO
        // https://alpha.data.banyan.computer/

        // Open this url with firefox
        open::with(
            format!(
                "{}/api/auth/device/register?spki={}&nonce={}",
                base_url, encoded_public_key, b64_nonce
            ),
            "firefox",
        )
        .expect("failed to open browser");

        // Now that the url has been opened, await the join handle
        match join_handle.await {
            Ok(Ok(response)) => {
                // Ok
                Ok(Self {
                    id: response.account_id,
                })
            }
            Ok(Err(err)) => Err(err),
            Err(err) => Err(ClientError::custom_error(&format!(
                "joining error: {}",
                err
            ))),
        }
    }

    /// Get the account associated with the current credentials in the Client
    pub async fn who_am_i(client: &mut Client) -> Result<Self, ClientError> {
        // Uhh we don't acutally need the ID for this one. There is probably a better pattern for this.
        let response: ReadWhoAmIResponse = client.call(ReadWhoAmI).await?;
        Ok(Self {
            id: response.account_id,
        })
    }

    /// Get the total usage for the account associated with the current credentials in the Client
    pub async fn usage(client: &mut Client) -> Result<u64, ClientError> {
        let response = client.call(GetTotalUsage).await?;
        Ok(response.size as u64)
    }

    /// Get the usage limit for the account associated with the current credentials in the Client
    pub async fn usage_limit(client: &mut Client) -> Result<u64, ClientError> {
        let response = client.call(GetUsageLimit).await?;
        Ok(response.size as u64)
    }
}

// TODO: wasm tests

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::banyan_api::client::Client;

    pub async fn authenticated_client() -> Client {
        let mut client = Client::new("http://localhost:3001", "http://localhost:3002").unwrap();
        let _ = Account::create_fake(&mut client).await.unwrap();
        client
    }

    pub async fn unauthenticated_client() -> Client {
        Client::new("http://localhost:3001", "http://localhost:3002").unwrap()
    }

    #[tokio::test]
    async fn who_am_i() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let subject = client.subject().unwrap();
        let read = Account::who_am_i(&mut client).await?;
        let subject_uuid = uuid::Uuid::parse_str(&subject).unwrap();
        assert_eq!(subject_uuid, read.id);
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn who_am_i_unauthenticated() {
        let mut client = unauthenticated_client().await;
        let _ = Account::who_am_i(&mut client).await.unwrap();
    }

    #[tokio::test]
    async fn usage() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let usage = Account::usage(&mut client).await?;
        assert_eq!(usage, 0);
        Ok(())
    }

    #[tokio::test]
    async fn usage_limit() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let usage_limit = Account::usage_limit(&mut client).await?;
        // 5 TiB
        assert_eq!(usage_limit, 5 * 1024 * 1024 * 1024 * 1024);
        Ok(())
    }

    #[tokio::test]
    async fn register_device() -> Result<(), ClientError> {
        let client = unauthenticated_client().await;
        let private_device_key = EcSignatureKey::generate().await.unwrap();
        // let public_key = private_key.public_key().unwrap();
        // let fingerprint = pretty_fingerprint(&public_key.fingerprint().await.unwrap());

        let account = Account::register_device(client, private_device_key).await?;
        println!("account: {:?}", account);

        Ok(())
    }
}
