use anyhow::Result;
use async_trait::async_trait;
use reqwest::Body;
use std::path::PathBuf;
use uuid::Uuid;

use crate::{
    banyan_api::{client::Client, error::ClientError},
    blockstore::carv2_memory::CarV2MemoryBlockStore,
};

use super::push::PushContent;

#[async_trait(?Send)]
pub trait UploadContent {
    fn get_hash(&self) -> Result<String>;
    async fn get_body(&self) -> Result<impl Into<Body>>;
    fn get_length(&self) -> Result<u64>;

    async fn upload(
        &self,
        host: Option<String>,
        metadata_id: Uuid,
        client: &mut Client,
    ) -> Result<(), ClientError> {
        client
            .multipart_no_content(PushContent {
                host_url: host.unwrap_or(client.remote_data.to_string()),
                metadata_id,
                content: self.get_body().await?.into(),
                content_len: self.get_length()?,
                content_hash: self.get_hash()?,
            })
            .await
    }
}

#[async_trait(?Send)]
impl UploadContent for PathBuf {
    fn get_hash(&self) -> anyhow::Result<String> {
        let reader = std::fs::File::open(self)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update_reader(&reader)?;
        Ok(hasher.finalize().to_string())
    }

    #[allow(refining_impl_trait)]
    async fn get_body(&self) -> anyhow::Result<tokio::fs::File> {
        Ok(tokio::fs::File::open(&self).await?)
    }

    fn get_length(&self) -> Result<u64> {
        Ok(self.metadata()?.len())
    }
}

#[async_trait(?Send)]
impl UploadContent for CarV2MemoryBlockStore {
    fn get_hash(&self) -> anyhow::Result<String> {
        let data = self.get_data();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&data);
        Ok(hasher.finalize().to_string())
    }

    #[allow(refining_impl_trait)]
    async fn get_body(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.get_data())
    }

    fn get_length(&self) -> Result<u64> {
        Ok(self.get_data().len() as u64)
    }
}

/*
let content = content_store.get_data();
hasher.update(&content);
let content = content_store.get_data();
let content_len = content.len() as u64;
let content_hash = hasher.finalize().to_string();
*/
