use std::{str::FromStr, thread, time::Duration};
use crate::banyan_api::{
    client::{Client, Credentials},
    error::ClientError,
    requests::core::{
        auth::{fake_account::create::*, who_am_i::read::*, device_api_key::regwait::start::{StartRegwait, StartRegwaitResponse}},
        buckets::usage::{GetTotalUsage, GetUsageLimit},
    },
    utils::generate_api_key,
};
use futures_core::Future;
use futures_util::FutureExt;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::{time::timeout, runtime::Handle};
use tomb_crypt::{prelude::{EcSignatureKey, PublicKey, PrivateKey}, pretty_fingerprint};
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
            .call_core(CreateFakeAccount { device_api_key_pem })
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
    pub async fn register_device(mut client: Client, private_device_key: EcSignatureKey) -> Result<Self, ClientError> {
        let public_device_key = private_device_key.public_key().expect("failed to create public key");
        // let public_device_key_fingerprint = pretty_fingerprint(&public_device_key.fingerprint().await.expect("unable to generate fingerprint"));
        // Public device key in PEM format 
        let public_device_key = String::from_utf8(public_device_key.export().await.expect("cant export key")).expect("cant convert key bytes to string");
        // Strip the public key of its new lines
        let mut stripped_public_key = public_device_key.replace('\n', "");
        // Strip the public key of its prefix and suffix
        stripped_public_key = stripped_public_key
        .strip_prefix("-----BEGIN PUBLIC KEY-----")
        .unwrap()
        .strip_suffix("-----END PUBLIC KEY-----")
        .unwrap()
        .to_string();

        // Represent the weird b64 characters with ones that are url-valid
        let encoded_public_key = stripped_public_key.replace('+', "-").replace('/', "_").replace('=', ".").to_string();
        println!("the stripped public key:\n ~{}~", stripped_public_key);
        println!("the encoded public key:\n ~{}~", encoded_public_key);
        
        // Start a background task

        // Create a new object for the registration wait task
        let start_regwait = StartRegwait::new();
        // Create a base64 url encoded version of the associated nonce
        let b64_nonce = base64_url::encode(&start_regwait.nonce.to_string());

        let timelimit = Duration::from_secs(5);

        let future = timeout(timelimit, async move {
            println!("calling core");
            let response: StartRegwaitResponse = client.call_core(start_regwait).await.expect("start_regwait failed");
            println!("finsihed calling core");
            response
        });

        let handle: tokio::task::JoinHandle<Result<StartRegwaitResponse, tokio::time::error::Elapsed>> = tokio::spawn(async move {
            future.await
        });


        // let closure = || { client.call_core(start_regwait) };

        // let future = Handle::current().block_on(client.call_core(start_regwait));
        // let handle = tokio::task::spawn(async move { 
            
        //     Handle::current().spawn_blocking(|| {
        //         client.call_core(start_regwait)  
        //     })
        // });

        // Handle::

        // let handle = local.run_until(async move {
        //     println!("i've done it!");
        //     // let mut client_1 = client.clone();
        //     // let future = client.call_core(start_regwait);
        //     let future = closure();
        //     let handle = tokio::task::spawn_local(future);
        //     println!("got the response");
        //     handle
        // });

        // let local_future = tokio::task::spawn_local(async move {
        //     println!("calling core");
        //     let response: StartRegwaitResponse = client_1.call_core(start_regwait).await.expect("start_regwait failed");
        //     println!("finsihed calling core");
        //     response
        // });

        // let handle = timeout(timelimit, );
        
        // local.run_until(async {}).await;
        
        // let mut rt = tokio::runtime::Runtime::new().unwrap();
        // let local = tokio::task::LocalSet::new();
        // let handle = local.run_until( async move {
            //     let response = tokio::task::spawn_local(async {
                //         println!("making request... ");
                //         let response: StartRegwaitResponse = client_1.call_core(start_regwait).await.expect("start_regwait failed");
                //         println!("got resp");
                //         response
                //     });
                //     response
                // });
                
                
                
        // Base url for the frontend
        let base_url = "http://127.0.0.1:3000";
        // Should be this in prod TODO
        // https://alpha.data.banyan.computer/
        

        
        // println!("opening url");
        
        // Open this url with firefox
        open::with(format!("{}/api/auth/device/register?spki={}&nonce={}", base_url, encoded_public_key, b64_nonce), "firefox").expect("failed to open browser");
        
        println!("url opened!");
        thread::sleep(Duration::SECOND);

                // handle

        // Now rejoin the future we spawned
        let result1 = handle.await;
        println!("result1: {:?}", result1);

        // Now rejoin the future we spawned
        let result2 = result1.unwrap();
        println!("result2: {:?}", result2);

        // // print!("please enter the account id:\n> ");
        // let mut account_id_string = String::new();
        // // std::io::stdin().read_line(&mut account_id_string).expect("Did not enter a correct string");
        // // println!("account_id_strin: {}", account_id_string);
        // let account_id = Uuid::from_str(&account_id_string.replace('\n', "")).expect("string was not a valid uuid");

        // // Update credentials
        // client.with_credentials(Credentials { account_id, signing_key: private_device_key });

        // Ok
        Ok(Self { id: Uuid::default() })
    }

    /// Get the account associated with the current credentials in the Client
    pub async fn who_am_i(client: &mut Client) -> Result<Self, ClientError> {
        // Uhh we don't acutally need the ID for this one. There is probably a better pattern for this.
        let response: ReadWhoAmIResponse = client.call_core(ReadWhoAmI).await?;
        Ok(Self {
            id: response.account_id,
        })
    }

    /// Get the total usage for the account associated with the current credentials in the Client
    pub async fn usage(client: &mut Client) -> Result<u64, ClientError> {
        let response = client.call_core(GetTotalUsage).await?;
        Ok(response.size as u64)
    }

    /// Get the usage limit for the account associated with the current credentials in the Client
    pub async fn usage_limit(client: &mut Client) -> Result<u64, ClientError> {
        let response = client.call_core(GetUsageLimit).await?;
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
        let mut client = unauthenticated_client().await;
        let private_device_key = EcSignatureKey::generate().await.unwrap();
        // let public_key = private_key.public_key().unwrap();
        // let fingerprint = pretty_fingerprint(&public_key.fingerprint().await.unwrap());

        let account = Account::register_device(client, private_device_key).await?;
        println!("account: {:?}", account);

        Ok(())
    }

    // #[tokio::test]
    // async fn regwait_start() -> Result<(), ClientError> {
    //     let mut client = unauthenticated_client().await;
    //     let private_key = EcSignatureKey::generate().await.unwrap();
    //     let public_key = private_key.public_key().unwrap();
    //     let fingerprint = pretty_fingerprint(&public_key.fingerprint().await.unwrap());

    //     // Call the start_regwait funtion
    //     let response = client.call_core(StartRegwait { fingerprint }).await?;

    //     println!("response: {:?}", response);

    //     Ok(())
    // }

}
