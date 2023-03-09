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
        unpack_plan::{ManifestData, UnpackPipelinePlan},
    },
    utils::fs as fsutil,
    vacuum,
};

/// Given the input directory, the output directory, the manifest file, and other metadata,
/// pack the input directory into the output directory and store a record of how this
/// operation was performed in the manifest file.
///
/// # Arguments
///
/// * `input_dir` - &Path representing the relative path of the input directory to pack.
/// * `output_dir` - &Path representing the relative path of where to store the packed data.
/// * `manifest_file` - &Path representing the relative path of where to store the manifest file.
/// * `chunk_size` - The maximum size of a packed file / chunk in bytes.
/// * `follow_links` - Whether or not to follow symlinks when packing.
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pack_pipeline(
    input_dir: &Path,
    output_dir: &Path,
    manifest_file: &Path,
    chunk_size: u64,
    follow_links: bool,
) -> Result<()> {
    // Construct the group config
    let group_config = create_group_config(input_dir, follow_links);

    // Create the output directory
    fsutil::ensure_path_exists_and_is_empty_dir(output_dir, false)
        .expect("output directory must exist and be empty");

    // This pack plan is used to construct FileGroup type PackPipelinePlans,
    // but is not unique to any individual file / FileGroup.
    let pack_plan = PackPlan {
        compression: CompressionScheme::new_zstd(),
        partition: PartitionScheme { chunk_size },
        encryption: EncryptionScheme::new(),
        writeout: output_dir.to_path_buf(),
    };

    /* Perform deduplication and plan how to copy the files */

    // Initialize a struct to figure out which files are friends with which
    let mut fclones_logger = fclones::log::StdLog::new();
    fclones_logger.no_progress = true;

    // TODO fix setting base_dir / do it right
    let file_groups = group_files(&group_config, &fclones_logger)?;
    // HashSet to track files that have already been seen
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    // Vector holding all the PackPipelinePlans for packing
    let mut packing_plan = vec![];
    // go over the files- do it in groups
    for group in file_groups {
        // Create a vector to hold the SpiderMetadata for each file in this group
        let mut metadatas = Vec::new();
        // For each file in this group
        for file in group.files {
            // Construct a PathBuf version of the path of this file
            let file_path_buf = file.path.to_path_buf();
            // Construct a canonicalized version of the path
            let canonicalized_path = file_path_buf.canonicalize().unwrap();
            // Insert that path into the list of seen paths
            seen_files.insert(canonicalized_path.clone());

            // Construct the original root and relative path
            let original_root = &group_config.base_dir;
            // Construct the original location relative to the root
            let original_location = file.path.strip_prefix(original_root).unwrap().to_path_buf();

            // Construct the metadata
            let spider_metadata = Arc::new(SpiderMetadata {
                /// This is the root of the backup
                original_root: original_root.to_path_buf(),
                /// This is the path relative to the root of the backup
                original_location,
                /// This is the canonicalized path of the original file
                canonicalized_path,
                /// This is the metadata of the original file
                original_metadata: fs::metadata(file_path_buf).unwrap(),
            });

            // Append the metadata
            metadatas.push(spider_metadata);
        }
        // Push a PackPipelinePlan with this file group
        packing_plan.push(PackPipelinePlan::FileGroup(metadatas, pack_plan.clone()));
    }

    /* Spider all the files so we can figure out what's there */
    // TODO fix setting follow_links / do it right
    let spidered: Vec<SpiderMetadata> =
        spider::spider(input_dir, group_config.follow_links).await?;

    // and now get all the directories and symlinks
    for spidered in spidered.into_iter() {
        // If this is a duplicate
        if seen_files.contains(&spidered.canonicalized_path.to_path_buf()) {
            // Just skip it
            continue;
        }
        // Now that we've checked for duplicates, add this to the seen files
        seen_files.insert(spidered.canonicalized_path.clone());

        // Construct Automatic Reference Counting pointer to the spidered metadata
        let origin_data = Arc::new(spidered.clone());
        // If this is a directory
        if spidered.original_metadata.is_dir() {
            // Push a PackPipelinePlan with this origin data
            packing_plan.push(PackPipelinePlan::Directory(origin_data));
        }
        // If this is a symlink
        else if spidered.original_metadata.is_symlink() {
            // Determine where this symlink points to, an operation that should never fail
            let symlink_target = fs::read_link(&spidered.canonicalized_path).unwrap();
            // Push a PackPipelinePlan with this origin data and symlink
            packing_plan.push(PackPipelinePlan::Symlink(origin_data, symlink_target));
        }
        // If this is a file that was not in a group
        else {
            // Push a PackPipelinePlan using fake file group of singular spidered metadata
            packing_plan.push(PackPipelinePlan::FileGroup(
                vec![origin_data],
                pack_plan.clone(),
            ));
        }
    }

    // TODO (laudiacay): For now we are doing compression in place, per-file. Make this better.
    let unpack_plans = futures::future::join_all(
        packing_plan
            .iter()
            .map(|copy_plan| vacuum::pack::do_pack_pipeline(copy_plan.clone())),
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

    // Construct the latest version of the ManifestData struct
    let manifest_data = ManifestData {
        version: env!("CARGO_PKG_VERSION").to_string(),
        unpack_plans,
    };

    // Use serde to convert the ManifestData to JSON and write it to the path specified
    // Return the result of this operation
    serde_json::to_writer_pretty(manifest_writer, &manifest_data).map_err(|e| anyhow::anyhow!(e))
}

/// Private function used to construct a GroupConfig struct from the relevant command line options.
/// This is used to make the main function more readable, as well as to ensure that
/// the GroupConfig options are always set correctly.
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
