use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use super::push::PushContent;
use crate::{
    banyan_api::{client::Client, error::ClientError},
    banyan_blockstore::CarV2MemoryBlockStore,
};

#[cfg(not(target_arch = "wasm32"))]
pub type ContentType = reqwest::Body;
#[cfg(target_arch = "wasm32")]
pub type ContentType = std::io::Cursor<Vec<u8>>;

#[async_trait(?Send)]
pub trait UploadContent {
    fn get_hash(&self) -> Result<String>;
    async fn get_body(&self) -> Result<ContentType>;
    fn get_length(&self) -> Result<u64>;

    async fn upload(
        &self,
        host_url: String,
        metadata_id: Uuid,
        client: &mut Client,
    ) -> Result<(), ClientError> {
        client
            .multipart_no_content(PushContent {
                host_url,
                metadata_id,
                content: self.get_body().await?,
                content_len: self.get_length()?,
                content_hash: self.get_hash()?,
            })
            .await
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
    async fn get_body(&self) -> anyhow::Result<ContentType> {
        #[cfg(target_arch = "wasm32")]
        return Ok(std::io::Cursor::new(self.get_data()));

        #[cfg(not(target_arch = "wasm32"))]
        return Ok(self.get_data().into());
    }

    fn get_length(&self) -> Result<u64> {
        Ok(self.get_data().len() as u64)
    }
}

#[cfg(test)]
#[cfg(feature = "fake")]
mod test {
    use crate::banyan_api::{
        error::ClientError, models::metadata::test::setup_and_push_metadata,
        requests::staging::upload::content::UploadContent,
    };
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn upload_content() -> Result<(), ClientError> {
        let mut setup = setup_and_push_metadata("upload_content").await?;
        // Create a grant and upload content
        setup
            .storage_ticket
            .clone()
            .create_grant(&mut setup.client)
            .await?;
        setup
            .content_store
            .upload(
                setup.storage_ticket.host.clone(),
                setup.metadata.id,
                &mut setup.client,
            )
            .await?;
        Ok(())
    }
}
