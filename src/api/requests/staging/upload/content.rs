use async_trait::async_trait;
use uuid::Uuid;

use super::push::PushContent;
use crate::{
    api::{client::Client, error::ApiError},
    blockstore::CarV2MemoryBlockStore,
};

#[cfg(not(target_arch = "wasm32"))]
pub type ContentType = reqwest::Body;
#[cfg(target_arch = "wasm32")]
pub type ContentType = std::io::Cursor<Vec<u8>>;

#[async_trait(?Send)]
pub trait UploadContent {
    type UploadError: From<ApiError>;

    fn get_hash(&self) -> Result<String, Self::UploadError>;
    async fn get_body(&self) -> Result<ContentType, Self::UploadError>;
    fn get_length(&self) -> Result<u64, Self::UploadError>;

    async fn upload(
        &self,
        host_url: String,
        metadata_id: Uuid,
        client: &mut Client,
    ) -> Result<(), Self::UploadError> {
        let push_content = PushContent {
            host_url,
            metadata_id,
            content: self.get_body().await?,
            content_len: self.get_length()?,
            content_hash: self.get_hash()?,
        };

        client
            .multipart_no_content(push_content)
            .await
            .map_err(|err| err.into())
    }
}

#[async_trait(?Send)]
impl UploadContent for CarV2MemoryBlockStore {
    type UploadError = ApiError;

    fn get_hash(&self) -> Result<String, Self::UploadError> {
        let data = self.get_data();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&data);
        Ok(hasher.finalize().to_string())
    }

    async fn get_body(&self) -> Result<ContentType, Self::UploadError> {
        #[cfg(target_arch = "wasm32")]
        return Ok(std::io::Cursor::new(self.get_data()));

        #[cfg(not(target_arch = "wasm32"))]
        return Ok(self.get_data().into());
    }

    fn get_length(&self) -> Result<u64, Self::UploadError> {
        Ok(self.get_data().len() as u64)
    }
}

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod test {
    use crate::api::{
        error::ApiError, models::metadata::test::setup_and_push_metadata,
        requests::staging::upload::content::UploadContent,
    };

    #[tokio::test]

    async fn upload_content() -> Result<(), ApiError> {
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
