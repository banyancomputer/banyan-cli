use anyhow::Result;
use fclones::{config::GroupConfig, group_files};
use std::{collections::HashSet, fs, path::PathBuf, sync::Arc};

use crate::{
    spider,
    types::{
        pack_plan::{PackPipelinePlan, PackPlan},
        shared::{CompressionScheme, EncryptionScheme, PartitionScheme},
        spider::SpiderMetadata,
        unpack_plan::UnpackPipelinePlan,
    },
    utils::fs as fsutil,
    vacuum,
};

pub async fn pack_pipeline(
    input_dir: PathBuf,
    output_dir: PathBuf,
    manifest_file: PathBuf,
    target_chunk_size: u64,
    mut group_config: GroupConfig,
) -> Result<()> {
    let output_dir = output_dir.canonicalize()?;

    fsutil::ensure_path_exists_and_is_empty_dir(&output_dir, false)
        .expect("output directory must exist and be empty");

    /* Spider all the files so we can figure out what's there */
    // TODO fix setting follow_links / do it right
    let spidered: Vec<SpiderMetadata> =
        spider::spider(&input_dir, group_config.follow_links).await?;

    /* Perform deduplication and plan how to copy the files */

    // Initialize a struct to figure out which files are friends with which
    let fclones_logger = fclones::log::StdLog::new();
    // TODO fix setting base_dir / do it right
    group_config.base_dir = input_dir.clone().into();
    let file_groups = group_files(&group_config, &fclones_logger)?;
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    let mut copy_plan = vec![];
    // go over the files- do it in groups
    for group in file_groups {
        copy_plan.push(PackPipelinePlan::FileGroup(
            group
                .files
                .iter()
                .map(|file| {
                    let file_path_buf = file.path.to_path_buf();
                    // TODO unwrap :|
                    let can_file_path_buf = file_path_buf.canonicalize().unwrap();
                    seen_files.insert(can_file_path_buf.clone());
                    Arc::new(SpiderMetadata {
                        /// this is the root of the backup
                        original_root: input_dir.clone(),
                        /// this is the path relative to the root of the backup
                        original_location: file_path_buf.clone(),
                        /// this is the canonicalized path of the original file
                        canonicalized_path: can_file_path_buf,
                        /// this is the metadata of the original file
                        original_metadata: fs::metadata(file_path_buf).unwrap(),
                    })
                })
                .collect::<Vec<Arc<SpiderMetadata>>>(),
            PackPlan {
                compression: CompressionScheme::new_zstd(),
                partition: PartitionScheme {
                    chunk_size: target_chunk_size,
                },
                encryption: EncryptionScheme::new(),
                writeout: output_dir.clone(),
            },
        ));
    }

    // and now get all the directories and symlinks
    for spidered in spidered.iter() {
        if seen_files.contains(&spidered.canonicalized_path.to_path_buf()) {
            continue;
        }
        let origin_data = Arc::new(spidered.clone());
        if spidered.original_metadata.is_dir() {
            // TODO clone spidered?? why
            copy_plan.push(PackPipelinePlan::Directory(Arc::new(spidered.clone())));
        } else if spidered.original_metadata.is_symlink() {
            let symlink_target = fs::read_link(spidered.canonicalized_path.clone()).unwrap();
            // TODO clone spidered?? why
            copy_plan.push(PackPipelinePlan::Symlink(
                origin_data.clone(),
                symlink_target,
            ));
        } else {
            panic!("files should be all done by now");
        }
        // TODO clone ?? why
        seen_files.insert(spidered.canonicalized_path.clone());
    }

    // TODO (laudiacay): For now we are doing compression in place, per-file. Make this better.
    let copied = futures::future::join_all(
        copy_plan
            .iter()
            .map(|copy_plan| vacuum::pack::do_file_pipeline(copy_plan.clone())),
    )
    .await
    .into_iter()
    .flat_map(|x| x.unwrap())
    .collect::<Vec<UnpackPipelinePlan>>();

    // For now just write out the content of compressed_and_encrypted to a file.
    // make sure the manifest file doesn't exist
    let manifest_writer = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(manifest_file)
        .unwrap();

    serde_json::to_writer_pretty(
        manifest_writer,
        // Iterate over the copied files and convert them to codable pipelines
        &copied,
    )
    .map_err(|e| anyhow::anyhow!(e))
}
