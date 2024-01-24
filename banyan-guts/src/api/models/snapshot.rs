use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

use crate::api::{
    client::Client, error::ApiError, models::metadata::Metadata,
    requests::core::buckets::snapshots::restore::RestoreSnapshot,
};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
/// Bucket Snapshot Definition
pub struct Snapshot {
    /// The unique identifier for the bucket metadata
    pub id: Uuid,
    /// The unique identifier for the bucket it belongs to
    pub bucket_id: Uuid,
    /// The unique identifier for the bucket it belongs to
    pub metadata_id: Uuid,
    /// The size of the data associated with the snapshot
    pub size: u64,
    /// The timestamp when the snapshot was created
    pub created_at: i64,
}

impl Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "\n{}\nsnapshot_id:\t{}\ndrive_id:\t{}\nmetadata_id:\t{}\ncreated_at:\t{}",
            "| SNAPSHOT INFO |".yellow(),
            self.id,
            self.bucket_id,
            self.metadata_id,
            self.created_at
        ))
    }
}

impl Snapshot {
    /// Restore a snapshot to its bucket
    pub async fn restore(&self, client: &mut Client) -> Result<Uuid, ApiError> {
        let request = RestoreSnapshot {
            bucket_id: self.bucket_id,
            snapshot_id: self.id,
        };
        let response = client.call(request).await?;
        Ok(response.metadata_id)
    }
    /// Get the metadata for this snapshot
    pub async fn metadata(&self, client: &mut Client) -> Result<Metadata, ApiError> {
        Metadata::read(self.bucket_id, self.metadata_id, client).await
    }
}

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod test {
    use std::{collections::BTreeSet, thread, time::Duration};

    use crate::{
        api::{
            error::ApiError,
            models::{
                account::test::authenticated_client, bucket::test::create_bucket,
                metadata::test::push_empty_metadata,
            },
        },
        prelude::api::{
            models::metadata::{Metadata, MetadataState},
            requests::core::buckets::snapshots::read::ReadAllSnapshots,
        },
    };

    #[tokio::test]
    #[ignore = "snapshot creation not yet finished"]
    async fn restore() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let (metadata, _, _) = push_empty_metadata(bucket.id, &mut client).await?;
        let snapshot_id = metadata.snapshot(BTreeSet::new(), &mut client).await?;

        thread::sleep(Duration::new(1, 0));

        println!("searching for snapshot_id {}", snapshot_id);

        // Create a Snapshot object after reading it down
        let snapshots = client
            .call(ReadAllSnapshots {
                bucket_id: bucket.id,
            })
            .await?;
        let snapshot = snapshots.0[0].to_snapshot(bucket.id);
        let restored_metadata_id = snapshot.restore(&mut client).await?;
        assert_eq!(restored_metadata_id, metadata.id);
        let restored_metadata =
            Metadata::read(bucket.id, restored_metadata_id, &mut client).await?;
        assert_eq!(restored_metadata.id, metadata.id);
        assert_eq!(metadata.bucket_id, restored_metadata.bucket_id);
        assert_eq!(restored_metadata.state, MetadataState::Current);
        Ok(())
    }
}
