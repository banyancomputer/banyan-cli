use crate::types::{
    pack_plan::PackPipelinePlan,
    spider::SpiderMetadata
};
use anyhow::Result;
use fclones::{config::GroupConfig, group_files};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::custom_fclones_logger::CustomLogger;

/// Creates a packing plan by grouping files from the input directory according to their similarities.
///
/// # Arguments
///
/// * `default_pack_plan` - A reference to the default PackPlan configuration.
/// * `input_dir` - A reference to the input directory path.
/// * `follow_links` - A boolean indicating whether to follow symbolic links.
/// * `seen_files` - A mutable reference to a HashSet of PathBuf containing paths of the seen files.
///
/// # Returns
///
/// * `Result<Vec<PackPipelinePlan>>` - A Result containing a vector of PackPipelinePlan objects
///   representing the packing plan, or an error in case of failure.
pub fn grouper(
    input_dir: &Path,
    follow_links: bool,
    seen_files: &mut HashSet<PathBuf>,
) -> Result<Vec<PackPipelinePlan>> {
    // Construct the group config
    let group_config = create_group_config(input_dir, follow_links);

    let file_groups = group_files(&group_config, &CustomLogger::default())?;
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
                /// This is the path relative to the root of the backup
                original_location,
                canonicalized_path,
                /// This is the metadata of the original file
                original_metadata: fs::metadata(file_path_buf).unwrap(),
            });

            // Append the metadata
            metadatas.push(spider_metadata);
        }
        // Push a PackPipelinePlan with this file group
        packing_plan.push(PackPipelinePlan::FileGroup(metadatas));
    }
    Ok(packing_plan)
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
