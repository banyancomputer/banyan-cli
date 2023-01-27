use std::path::PathBuf;
use anyhow::{anyhow, Result};
use crate::types::pipeline::{PipelineToDisk};



pub async fn unpack_pipeline(input_dir: PathBuf, manifest_file: PathBuf, output_dir: PathBuf) -> Result<()> {
    // parse manifest file into Vec<PipelineToDisk>
    let reader = std::fs::File::open(manifest_file)?;
    let pipelines: Vec<PipelineToDisk> = serde_json::from_reader(reader)?;

    // tokio_stream::iter(pipelines)
    //     .then()

}
