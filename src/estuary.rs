use banyan_shared::{
    types::*,
};
use anyhow::{Result, Error};
use std::env::var;
use serde_json::{Map, Value};
use serde::{Deserialize, Deserializer};
use cid::Cid;
use reqwest::{multipart, Body, Client};
use tokio_util::codec::{BytesCodec, FramedRead};

/// Content - What's returned from the Estuary API /content/stats endpoint
#[derive(Debug, Deserialize)]
pub struct Content {
    id: u32,
    #[serde(rename = "cid", deserialize_with = "des_cid_from_map")]
    cid_str:      String,
    #[serde(rename = "dealId")]
    deal_id:   u32,
    name: String,
    size:     u32,
}

// Note (al) - The Estuary API returns a CID as a map with a "/" key
pub fn des_cid_from_map<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let map: Map<String, Value> = Map::deserialize(deserializer).unwrap();
    let cid_str = map.get("/").unwrap().as_str().unwrap();
    Ok(cid_str.to_string())
}

/// EstuaryClient - A struct for managing Requests to an Estuary API
pub struct EstuaryClient {
    /// The Estuary API Hostname
    pub estuary_api_hostname: String,
    /// The Estuary API Key
    pub estuary_api_key: Option<String>,
}

impl Default for EstuaryClient {
    /// Create a new EstuaryClient from the Environment
    /// ```no_run
    /// use crate::estuary::EstuaryClient;
    /// let estuary_client = EstuaryClient::default();
    /// ```
    /// # Panics
    /// This function will panic if the `ESTUARY_API_HOSTNAME` environment variable is not set.
    fn default() -> Self {
        Self {
            estuary_api_hostname: var("ESTUARY_API_HOSTNAME").unwrap_or_else(|_| {
                panic!("ESTUARY_API_HOSTNAME environment variable is not set")
            }),
            estuary_api_key: var("ESTUARY_API_KEY").ok(),
        }
    }
}

// TODO: Should I be initializing a ReqWest Client here, or is ok to do it in each function?
impl EstuaryClient {
    /// Create a new EstuaryClient using custom values
    /// # Arguments
    /// * `estuary_api_hostname` - The Hostname of the Estuary API to use.
    /// * `estuary_api_key` - The (optional) API Key to use for the Estuary API.
    /// ```no_run
    /// use crate::estuary::EstuaryClient;
    /// let estuary_client = EstuaryClient::new("http://localhost:3004".to_string(), None);
    /// ```
    /// # Panics
    /// This function should not panic.
    /// Misconfiguration will result in an error when making requests.
    pub fn new(
        estuary_api_hostname: String,
        estuary_api_key: Option<String>,
    ) -> Self {
        Self {
            estuary_api_hostname,
            estuary_api_key,
        }
    }

    /* Struct Methods */

    /// Get the Estuary API Hostname
    pub fn get_estuary_api_hostname(&self) -> String {
        self.estuary_api_hostname.clone()
    }

    /// Stage a File on Estuary
    /// # Arguments
    /// * `file` - The handle to the file to stage
    /// * `deal_id` - The Deal ID to use for the file
    /// * `b3_hash` - The Blake3 Hash of the file
    /// ```no_run
    /// use crate::estuary::EstuaryClient;
    /// use banyan_shared::types::*
    /// use blake3::Hash
    ///
    /// let client = EstuaryClient::default();
    /// client.stage_file(
    ///     tokio::fs::File::open("path_to_file.txt").await?,
    ///     DealID(0),
    ///     Blake3HashToken(Hash::from_str("b3_hash").unwrap()),
    /// );
    /// ```
    /// # Panics
    /// * If there is an error reading the file
    /// * If there is an error sending the request
    /// Stage a File on Estuary
    /// # Arguments
    /// * `file_path` - The path to the file to stage
    /// * `deal_id_str` - The Deal ID to use for the file, as a String
    /// * `b3_hash_str` - The Blake3 Hash of the file, as a Hex String
    /// # Returns
    /// * `Result<(), Error>` - Errors if there is an error staging the file
    pub async fn stage_file(
        &self,
        file_path: String,
        deal_id_str: String,
        b3_hash_str: String,
    ) -> Result<(), Error> {
        if self.estuary_api_key.is_none() {
            panic!("No Estuary API Key is set");
        }
        let estuary_api_key = self.estuary_api_key.clone().unwrap();
        // Initialize an HTTP Client
        let client = Client::new();
        // Read the File as a Tokio File
        let file = tokio::fs::File::open(&file_path).await?;
        // Read file body stream
        let file_body = Body::wrap_stream(FramedRead::new(file, BytesCodec::new()));
        // Define a Form Part for the File
        let some_file = multipart::Part::stream(file_body)
            .file_name(file_path)
            .mime_str("text/plain")?;
        // Create the multipart form
        let form = multipart::Form::new()
            .part("data", some_file) //add the file part
            .text("dealId", deal_id_str) //add the dealId
            .text("blake3Hash", b3_hash_str); //add the b3Hash
        // Initialize the Request
        let res = client
            // POST to the /content/add endpoint
            .post(format!("{}/content/add", self.estuary_api_hostname))
            // Set the Authorization Header
            .header("Authorization", format!("Bearer {}", estuary_api_key))
            // Add the Form
            .multipart(form)
            // Send the Request
            .send()
            // Await the Response
            .await?;
        // Check the Status Code
        if res.status().is_success() {
            // No Need to listen to the Response - We're good!
            Ok(())
        } else {
            Err(Error::msg(format!(
                "Error staging file: {}",
                res.status().as_str()
            )))
        }
    }

    /// Get the First 500 pieces of Content from Estuary
    /// ```no_run
    /// use crate::estuary::EstuaryClient;
    /// let client = EstuaryClient::default();
    /// let content = client.get_content().await?;
    /// ```
    /// # Panics
    /// * If there is an error sending the request
    /// * If there is an error parsing the response
    pub async fn get_content(&self) -> Result<Vec<Content>, Error> {
        if self.estuary_api_key.is_none() {
            panic!("No Estuary API Key is set");
        }
        let estuary_api_key = self.estuary_api_key.clone().unwrap();
        // Initialize an HTTP Client
        let client = Client::new();
        // Initialize the Request
        let res = client
            // GET to the /content endpoint
            .get(format!("{}/content/stats", self.estuary_api_hostname))
            // Set the Authorization Header
            .header("Authorization", format!("Bearer {}", estuary_api_key))
            // Send the Request
            .send()
            // Await the Response
            .await?;
        // Check the Status Code
        if res.status().is_success() {
            // Parse the Response
            // dbg!(&res.json().await?);
            let content: Vec<Content> = res.json().await?;
            // dbg!(&content);\
            // Print the response body as json
            Ok(content)
            // Ok(Vec::new())
        } else {
            Err(Error::msg(format!(
                "Error getting content: {}",
                res.status().as_str()
            )))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    /// Test that we can create a BanyanClient from the Environment
    fn default_client() {
        let _client = EstuaryClient::default();
        return;
    }

    #[tokio::test]
    /// Try to stage a file on Estuary with a fake DealId
    async fn stage_file() {
        let client = EstuaryClient::default();
        let deal_id_str = "0".to_string();
        let b3_hash_str = "9a38ad06b076d7617291f50adb6ea857281dc0b75374ed86122105178afed119".to_string();
        client
            .stage_file("Cargo.toml".to_string(), deal_id_str, b3_hash_str)
            .await
            .unwrap();
        return;
    }

    #[tokio::test]
    /// Try to get content from Estuary
    async fn get_content() {

        let client = EstuaryClient::default();
        let content: Vec<Content> = client.get_content().await.unwrap();
        dbg!(content);
        return;
    }
}
