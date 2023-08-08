use anyhow::Result;
use reqwest::Url;
use super::{request::Requestable, error::ClientError};

pub struct Client {
    remote: Url
    // token: Token
}

impl Client {
    pub fn new(remote: &str) -> Result<Self> {
        Ok(Self {
            remote: Url::parse(remote)?
        })
    }

    pub async fn send<R: Requestable>(&self, request: R) -> Result<R::ResponseType, ClientError> {
        // Determine the full URL to send the request to 
        // This should never fail
        let full_url = self.remote.join(&request.endpoint()).unwrap();

        println!("the full_url is {:?}", full_url);

        // Create a new client
        let client = reqwest::Client::new();
        // Create the RequestBuilder
        let builder = client
            .request(request.method(), full_url)
            .json(&request);
        // Send and await the response
        let response = builder.send().await.map_err(ClientError::http_error)?;
        // If we succeeded
        if response.status().is_success() {
            response
                .json::<R::ResponseType>()
                .await
                .map_err(ClientError::bad_format)
        } else {
            let err = response
                .json::<R::ErrorType>()
                .await
                .map_err(ClientError::bad_format)?;

            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ClientError::from(err))
        }
    }
}