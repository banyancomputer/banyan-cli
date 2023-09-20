use anyhow::Result;
use async_recursion::async_recursion;
use std::{fs::File, io::Write, path::Path, rc::Rc};
use tomb_common::{blockstore::RootedBlockStore, metadata::FsMetadata};
use wnfs::{
    common::BlockStore,
    private::{PrivateForest, PrivateNode},
};

use crate::pipelines::error::TombError;

use super::spider::path_to_segments;

#[async_recursion(?Send)]
/// Recursively reconstruct each file and directory from the WNFS to disk
pub async fn process_node(
    fs: &mut FsMetadata,
    metadata_store: &impl RootedBlockStore,
    content_store: &impl RootedBlockStore,
    extracted: &Path,
    built_path: &Path,
) -> Result<()> {
    let path_segments = path_to_segments(built_path)?;
    let result = if path_segments.len() == 0 {
        Ok(Some(fs.root_dir.as_node()))
    } else {
        fs.get_node(path_segments, metadata_store).await
    };
    // Match that result
    match result {
        Ok(Some(PrivateNode::Dir(dir))) => {
            println!("{} was a dir", built_path.display());
            // Create the directory we are in
            std::fs::create_dir_all(extracted.join(built_path))?;
            // Obtain a list of this Node's children
            let node_names: Vec<&String> = dir.get_entries().collect();
            // For each of those children
            for node_name in node_names {
                // Full path of the node in question
                let built_path = &built_path.join(node_name);
                // Process that node, too
                process_node(fs, metadata_store, content_store, extracted, built_path).await?;
            }
        }
        Ok(Some(PrivateNode::File(_))) => {
            println!("{} was a file", built_path.display());
            let file_path = &extracted.join(built_path);
            // This is where the file will be extracted no matter what
            if let Ok(content) = fs
                .read(
                    path_to_segments(&built_path)?,
                    metadata_store,
                    content_store,
                )
                .await
            {
                // // If this file is a symlink
                // if let Some(path) = file.symlink_origin() {
                //     println!("file was symlink :3");
                //     // Write out the symlink
                //     symlink(output_dir.join(path), file_path)?;
                //     Ok(())
                // }
                // If this is a real file, try to read in the content
                println!("file was contented :3");
                // Create the file at the desired location
                let mut output_file = File::create(file_path)?;
                // Write out the content to disk
                output_file.write_all(&content)?;
            } else {
                // return Err(TombError::file_missing_error(file_path.to_path_buf()))
                return Err(anyhow::anyhow!("file missing error"));
            }
        }
        Ok(None) => {
            return Err(TombError::file_missing_error(built_path.to_path_buf()).into());
        }
        Err(err) => {
            return Err(anyhow::anyhow!("rrro!!"));
        }
    }
    Ok(())
}
