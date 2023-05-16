use anyhow::Result;
use std::{collections::HashSet, path::Path, rc::Rc};
use wnfs::private::PrivateForest;

use crate::{
    types::blockstore::networkblockstore::NetworkBlockStore,
    utils::pipeline::{load_forest, load_manifest},
};

/// Takes locally packed car file data and throws it onto a server
pub async fn pull_pipeline(
    tomb_path: &Path,
    output_dir: &Path,
    store: &NetworkBlockStore,
) -> Result<()> {
    info!("🎉 Nice! A copy of the remote encrypted filesystem now exists locally.");

    let manifest = load_manifest(tomb_path).await?;
    let forest = load_forest(&manifest).await?;

    // let empty_forest = Rc::new(PrivateForest::new());
    // let differences = forest.diff(&empty_forest, &manifest.content_store).await?;
    // let mut all_cids = HashSet::new();
    // for difference in differences {
    //     if let Some(difference1) = difference.value1 {
    //         all_cids.extend(difference1);
    //     }
    //     if let Some(difference2) = difference.value2 {
    //         all_cids.extend(difference2);
    //     }
    // }

    // println!("all CIDs: {:?}", all_cids);

    Ok(())
}
