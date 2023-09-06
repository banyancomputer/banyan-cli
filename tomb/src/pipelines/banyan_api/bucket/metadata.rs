use crate::{
    cli::command::MetadataSubCommand, pipelines::error::TombError,
    types::config::globalconfig::GlobalConfig, utils::wnfsio::compute_directory_size,
};
use anyhow::anyhow;
use futures_util::StreamExt;
use tomb_common::{
    banyan_api::{client::Client, error::ClientError, models::metadata::*},
    blockstore::RootedBlockStore,
    metadata::FsMetadata,
};

pub(crate) async fn pipeline(
    global: &GlobalConfig,
    client: &mut Client,
    command: MetadataSubCommand,
) -> Result<String, TombError> {
    match command {
        MetadataSubCommand::Read {
            bucket_specifier,
            metadata_id,
        } => {
            // Get Bucket config
            let config = global.get_bucket_by_specifier(&bucket_specifier)?;
            // If we can get the metadata
            if let Some(remote_id) = config.remote_id {
                Metadata::read(remote_id, metadata_id, client)
                    .await
                    .map(|metadata| format!("{:?}", metadata))
                    .map_err(TombError::client_error)
            } else {
                Err(TombError::anyhow_error(anyhow!(
                    "Conffig has no remote id!"
                )))
            }
        }
        MetadataSubCommand::Push(bucket_specifier) => {
            // Get info
            let wrapping_key = global.wrapping_key().await?;
            let config = global.get_bucket_by_specifier(&bucket_specifier)?;
            let fs = FsMetadata::unlock(&wrapping_key, &config.metadata).await?;
            let valid_keys = fs.share_manager.public_pems();

            let expected_data_size = compute_directory_size(&config.metadata.path)? as u64;
            let bucket_id = config.remote_id.expect("no remote id");
            let root_cid = config.metadata.get_root().expect("no root cid").to_string();
            let metadata_stream = tokio::fs::File::open(&config.metadata.path).await?;
            // Push the Metadata
            Metadata::push(
                bucket_id,
                root_cid,
                expected_data_size,
                valid_keys,
                metadata_stream,
                client,
            )
            .await
            .map(|(metadata, storage_ticket)| {
                let mut info = format!("\t{}", metadata);
                if let Some(storage_ticket) = storage_ticket {
                    info.push_str(&format!("\n\n\t{}", storage_ticket))
                }
                info
            })
            .map_err(TombError::client_error)
        }
        MetadataSubCommand::ReadCurrent(bucket_specifier) => {
            let config = global.get_bucket_by_specifier(&bucket_specifier)?;
            let bucket_id = config.remote_id.expect("no remote id");
            Metadata::read_current(bucket_id, client)
                .await
                .map(|metadata| format!("{:?}", metadata))
                .map_err(TombError::client_error)
        }
        MetadataSubCommand::List(bucket_specifier) => {
            let config = global.get_bucket_by_specifier(&bucket_specifier)?;
            let bucket_id = config.remote_id.expect("no remote id");
            Metadata::read_all(bucket_id, client)
                .await
                .map(|metadatas| {
                    metadatas
                        .iter()
                        .fold("<< METADATAS >>".to_string(), |acc, metadata| {
                            format!("{}{}", acc, metadata)
                        })
                })
                .map_err(TombError::client_error)
        }
        MetadataSubCommand::Pull {
            bucket_specifier,
            metadata_id,
        } => {
            let config = global.get_bucket_by_specifier(&bucket_specifier)?;
            let bucket_id = config.remote_id.expect("no remote id");
            let metadata = Metadata::read(bucket_id, metadata_id, client).await?;
            let mut byte_stream = metadata.pull(client).await?;
            let mut file = tokio::fs::File::create(&config.metadata.path).await?;

            println!("starting to download metadata...");

            while let Some(chunk) = byte_stream.next().await {
                tokio::io::copy(
                    &mut chunk.map_err(ClientError::http_error)?.as_ref(),
                    &mut file,
                )
                .await?;
            }

            Ok(format!("successfully downloaded metadata"))
        }
        MetadataSubCommand::Snapshot {
            bucket_specifier,
            metadata_id,
        } => {
            let config = global.get_bucket_by_specifier(&bucket_specifier)?;
            let bucket_id = config.remote_id.expect("no remote id");
            let metadata = Metadata::read(bucket_id, metadata_id, client).await?;

            metadata
                .snapshot(client)
                .await
                .map(|snapshot| format!("{:?}", snapshot))
                .map_err(TombError::client_error)
        }
    }
}
