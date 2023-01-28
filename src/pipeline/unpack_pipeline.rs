use crate::types::pipeline::PipelineToDisk;
use crate::vacuum::unpack::do_file_pipeline;
use anyhow::Result;
use std::path::PathBuf;
use tokio_stream::StreamExt;

pub async fn unpack_pipeline(
    input_dir: PathBuf,
    manifest_file: PathBuf,
    output_dir: PathBuf,
) -> Result<()> {
    // parse manifest file into Vec<PipelineToDisk>
    let reader = std::fs::File::open(manifest_file)?;
    let pipelines: Vec<PipelineToDisk> = serde_json::from_reader(reader)?;

    tokio_stream::iter(pipelines)
        .then(|pipeline_to_disk| {
            do_file_pipeline(pipeline_to_disk, input_dir.clone(), output_dir.clone())
        })
        .collect::<Result<Vec<_>>>()
        .await?;
    Ok(())
}
