/// Requeest for retrieveing authorization
pub mod authorization;
/// Request for creating a new storage grant on the staging area.
pub mod create;

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod test {
    use crate::api::{
        error::ApiError, models::metadata::test::setup_and_push_metadata,
        requests::staging::upload::content::UploadContent,
    };

    #[tokio::test]

    async fn create_grant() -> Result<(), ApiError> {
        let mut setup = setup_and_push_metadata("create_grant").await?;
        setup.storage_ticket.create_grant(&mut setup.client).await?;
        Ok(())
    }

    #[tokio::test]

    async fn authorization_grants() -> Result<(), ApiError> {
        let mut setup = setup_and_push_metadata("authorization_grants").await?;
        // Create a grant
        setup.storage_ticket.create_grant(&mut setup.client).await?;
        // Assert 404 before any space has been allocated
        assert!(setup
            .bucket
            .get_grants_token(&mut setup.client)
            .await
            .is_err());
        // Upload content
        setup
            .content_store
            .upload(
                setup.storage_ticket.host,
                setup.metadata.id,
                &mut setup.client,
            )
            .await?;

        // Sleep to allow upload report to be processed
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Successfully get a new bearer token which can access the new grants
        setup.bucket.get_grants_token(&mut setup.client).await?;
        Ok(())
    }
}
