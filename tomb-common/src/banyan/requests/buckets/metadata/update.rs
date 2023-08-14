// use std::error::Error;
// use std::fmt::{self, Display, Formatter};

// use reqwest::{Client, RequestBuilder, Url};
// use serde::{Deserialize, Serialize};
// use uuid::Uuid;

// use crate::requests::{ApiRequest, MetadataState};

// #[derive(Debug)]
// pub struct PublishBucketMetadata<S>
// where
//     reqwest::Body: From<S>,
// {
//     pub bucket_id: Uuid,

//     pub expected_data_size: usize,
//     pub metadata_cid: String,
//     pub root_cid: String,

//     pub metadata_stream: S,
// }

// impl<S> ApiRequest for PublishBucketMetadata<S>
// where
//     reqwest::Body: From<S>,
// {
//     type ResponseType = PublishBucketMetadataResponse;
//     type ErrorType = PublishBucketMetadataError;

//     fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
//         let pbm_req = PublishBucketMetadataRequest {
//             data_size: self.expected_data_size,
//             metadata_cid: self.metadata_cid,
//             root_cid: self.root_cid,
//         };

//         let url = base_url
//             .join(format!("/api/v1/buckets/{}/publish", self.bucket_id).as_str())
//             .unwrap();

//         let multipart_json_data = serde_json::to_string(&pbm_req).unwrap();
//         let multipart_json =
//             reqwest::multipart::Part::bytes(multipart_json_data.as_bytes().to_vec())
//                 .mime_str("application/json")
//                 .unwrap();

//         let multipart_car = reqwest::multipart::Part::stream(self.metadata_stream)
//             .mime_str("application/vnd.ipld.car; version=2")
//             .unwrap();

//         let multipart_form = reqwest::multipart::Form::new()
//             .part("request-data", multipart_json)
//             .part("car-upload", multipart_car);

//         client.post(url).multipart(multipart_form)
//     }

//     fn requires_authentication(&self) -> bool {
//         true
//     }
// }

// #[derive(Debug, Serialize)]
// struct PublishBucketMetadataRequest {
//     data_size: usize,
//     metadata_cid: String,
//     root_cid: String,
// }

// #[derive(Debug, Deserialize)]
// pub struct PublishBucketMetadataResponse {
//     pub id: Uuid,
//     pub state: MetadataState,

//     pub storage_host: String,
//     pub storage_authorization: String,
// }

// #[derive(Debug, Deserialize)]
// #[non_exhaustive]
// pub struct PublishBucketMetadataError {
//     #[serde(rename = "error")]
//     kind: PublishBucketMetadataErrorKind,
// }

// impl Display for PublishBucketMetadataError {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         use PublishBucketMetadataErrorKind::*;

//         let msg = match &self.kind {
//             Unknown => "an unknown error occurred publishing the metadata",
//         };

//         f.write_str(msg)
//     }
// }

// impl Error for PublishBucketMetadataError {}

// #[derive(Debug, Deserialize)]
// #[non_exhaustive]
// #[serde(tag = "type", rename_all = "snake_case")]
// enum PublishBucketMetadataErrorKind {
//     Unknown,
// }
