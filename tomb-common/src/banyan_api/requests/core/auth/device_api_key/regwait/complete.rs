use crate::banyan_api::requests::ApiRequest;
use serde::Deserialize;
use std::error::Error;
use uuid::Uuid;

#[derive(Debug)]
pub struct CompleteRegwait {
    pub nonce: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CompleteRegwaitResponse {
    pub account_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CompleteRegwaitError {
    msg: String,
}

impl std::fmt::Display for CompleteRegwaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for CompleteRegwaitError {}

impl ApiRequest for CompleteRegwait {
    type ResponseType = CompleteRegwaitResponse;
    type ErrorType = CompleteRegwaitError;

    fn build_request(
        self,
        base_url: &reqwest::Url,
        client: &reqwest::Client,
    ) -> reqwest::RequestBuilder {
        todo!()
    }

    fn requires_authentication(&self) -> bool {
        todo!()
    }
}
