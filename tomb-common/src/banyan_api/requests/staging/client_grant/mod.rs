/// Requeest for retrieveing authorization
pub mod authorization;
/// Request for creating a new storage grant on the staging area.
pub mod create;

#[cfg(test)]
mod test {
    use serial_test::serial;

    use crate::banyan_api::{
        error::ClientError, models::metadata::test::setup_and_push_metadata,
        requests::staging::upload::content::UploadContent,
    };

    #[tokio::test]
    #[serial]
    async fn create_grant() -> Result<(), ClientError> {
        let mut setup = setup_and_push_metadata("create_grant").await?;
        setup.storage_ticket.create_grant(&mut setup.client).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn authorization_grants() -> Result<(), ClientError> {
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
        // Successfully get a new bearer token which can access the new grants
        setup.bucket.get_grants_token(&mut setup.client).await?;
        Ok(())
    }
}
