use anyhow::{anyhow, Result};
use std::path::Path;

/// Check if a path is an existing directory
pub fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        // create path if it doesn't exist
        std::fs::create_dir_all(path)?;
    }
    if !path.is_dir() {
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }
    Ok(())
}

/// Check if a path is an existing empty directory
pub fn ensure_path_exists_and_is_empty_dir(path: &Path) -> Result<()> {
    ensure_path_exists_and_is_dir(path)?;
    if path.read_dir().unwrap().count() > 0 {
        return Err(anyhow!("Path is not empty: {}", path.display()));
    }
    Ok(())
}

// Note (amiller68): The following is not used.
/*
   // TODO (laudiacay): Use proper function names
   // this comment lies in memoriam of the time i set these both to 10. if you estimate the disk
   // space used by a directory as only 512 bits, this would have filled 5 terabytes of disk space.
   // i'm not sure what i was thinking.
   pub fn make_big_filesystem_clusterfuck(depth_to_go: usize, width: usize, cwd: PathBuf) {
       if depth_to_go == 0 {
           for i in 0..width {
               let mut path = cwd.clone();
               path.push(format!("file{i}"));
               std::fs::File::create(path).unwrap();
           }
       } else {
           for i in 0..width {
               let mut path = cwd.clone();
               path.push(format!("dir{i}"));
               std::fs::create_dir(path.clone()).unwrap();
               make_big_filesystem_clusterfuck(depth_to_go - 1, width, path);
           }
       }
   }
*/

// TODO (xBalbinus & thea-exe) Our inline tests. Coordinate with laudiacay on what we need here
#[cfg(test)]
mod test {
    // Note (amiller68): Commented out seemingly irrelevant test
    // #[tokio::test]
    // async fn test_copy_paths_recursively() {
    //     use super::*;
    //     let tmp = tempfile::tempdir().unwrap();
    //     let scratch_root = tmp.path().join("scratch");
    //     let og_root = tmp.path().join("og");
    //     std::fs::create_dir(&scratch_root).unwrap();
    //     std::fs::create_dir(&og_root).unwrap();
    //     make_big_filesystem_clusterfuck(3, 3, og_root.clone());
    //     let paths = std::fs::read_dir(og_root.clone())
    //         .unwrap()
    //         .map(|res| res.unwrap().path())
    //         .collect::<Vec<PathBuf>>();
    //
    //     let stream = copy_paths_recursively(paths, scratch_root.clone(), false).await;
    //     let out_files = stream
    //         .map(|res| res.1.unwrap())
    //         .collect::<Vec<PathBuf>>()
    //         .await;
    //
    //     let in_files = FilesystemIterator::new(og_root, false)
    //         .await
    //         .collect::<Vec<PathBuf>>()
    //         .await;
    //
    //     assert_eq!(in_files.len() - 1, out_files.len());
    //     for file in out_files {
    //         let stripped =
    //             Path::new("/").join(file.strip_prefix(&scratch_root).unwrap().to_path_buf());
    //         assert!(in_files.contains(&stripped));
    //     }
    // }
}
