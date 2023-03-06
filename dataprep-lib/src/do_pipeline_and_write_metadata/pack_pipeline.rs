use anyhow::Result;
use fclones::{config::GroupConfig, group_files};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

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
    follow_links: bool,
) -> Result<()> {
    // Construct the group config
    let group_config = create_group_config(&input_dir, follow_links);

    // Create the output directory
    fsutil::ensure_path_exists_and_is_empty_dir(&output_dir, false)
        .expect("output directory must exist and be empty");

    /* Spider all the files so we can figure out what's there */
    // TODO fix setting follow_links / do it right
    let spidered: Vec<SpiderMetadata> =
        spider::spider(&input_dir, group_config.follow_links).await?;

    /* Perform deduplication and plan how to copy the files */

    // Initialize a struct to figure out which files are friends with which
    let mut fclones_logger = fclones::log::StdLog::new();
    fclones_logger.no_progress = true;

    // TODO fix setting base_dir / do it right
    let file_groups = group_files(&group_config, &fclones_logger)?;
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    let mut copy_plan = vec![];
    // go over the files- do it in groups
    for group in file_groups {
        let mut metadatas = vec![];
        for file in group.files {
            let file_path_buf = file.path.to_path_buf();
            let can_file_path_buf = file_path_buf.canonicalize().unwrap();
            seen_files.insert(can_file_path_buf.clone());

            // Construct the original root and relative path
            let original_root = group_config.base_dir.clone();
            let original_location = file.path.strip_prefix(&original_root).unwrap();

            // Construct the metadata
            let spider_metadata = Arc::new(SpiderMetadata {
                /// this is the root of the backup
                original_root: original_root.to_path_buf(),
                /// this is the path relative to the root of the backup
                original_location: original_location.to_path_buf(),
                /// this is the canonicalized path of the original file
                canonicalized_path: can_file_path_buf,
                /// this is the metadata of the original file
                original_metadata: fs::metadata(file_path_buf).unwrap(),
            });

            // Append the metadata
            metadatas.push(spider_metadata);
        }
        let pack_plan = PackPlan {
            compression: CompressionScheme::new_zstd(),
            partition: PartitionScheme {
                chunk_size: target_chunk_size,
            },
            encryption: EncryptionScheme::new(),
            writeout: output_dir.clone(),
        };
        copy_plan.push(PackPipelinePlan::FileGroup(metadatas, pack_plan));
    }

    // and now get all the directories and symlinks
    for spidered in spidered.iter() {
        // If this is a duplicate
        if seen_files.contains(&spidered.canonicalized_path.to_path_buf()) {
            // Just skip it
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
        }
        // These files are not in a file group but still need to be processed!
        else {
            // Construct an artificial file group
            let magic_spider = vec![Arc::new(spidered.clone())];

            // Create pack pipeline plan
            let pack_plan = PackPlan {
                compression: CompressionScheme::new_zstd(),
                partition: PartitionScheme {
                    chunk_size: target_chunk_size,
                },
                encryption: EncryptionScheme::new(),
                writeout: output_dir.clone(),
            };
            copy_plan.push(PackPipelinePlan::FileGroup(magic_spider, pack_plan));
        }
        // TODO clone ?? why
        seen_files.insert(spidered.canonicalized_path.clone());
        // println!("inserted {:?} into seen_files", spidered.canonicalized_path);
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

fn create_group_config(input_dir: &Path, follow_links: bool) -> GroupConfig {
    let base_dir = input_dir.canonicalize().unwrap();

    // we checked over these options manually and sorted them
    GroupConfig {
        // will definitely never need to change
        output: None,
        format: Default::default(),
        stdin: false,
        isolate: false, // TODO laudiacay um bug?
        in_place: false,
        no_copy: false,
        rf_over: None,
        rf_under: None,
        unique: false,

        // will probably never need to change
        depth: None,
        match_links: false,
        symbolic_links: false, // TODO laudiacay here be bugs
        transform: None,
        min_size: (0_usize).into(),
        max_size: None,
        ignore_case: false,
        regex: false,

        // may want to change for feature adds in the future
        hidden: true,
        no_ignore: false, // TODO laudiacay HELPPPP THIS MIGHT BE BUGS
        // TODO laudiacay ????
        name_patterns: vec![],
        path_patterns: vec![],
        exclude_patterns: vec![],
        hash_fn: Default::default(),
        cache: false,

        // we are using this option it is load bearing
        threads: vec![(
            "default".to_string().parse().unwrap(),
            fclones::config::Parallelism {
                random: 1,
                sequential: 1,
            },
        )],
        follow_links,
        base_dir: base_dir.into(),
        paths: vec![".".into()],
    }
    // TODO think about fclones caching for repeated runs :3 this will b useful for backup utility kind of thing
    // TODO groupconfig.threads and think about splitting squential and random io into separate thread pools
}
