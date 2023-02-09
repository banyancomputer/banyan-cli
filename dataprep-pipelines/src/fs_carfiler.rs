// TODO (laudiacay): i think i'm willing to bet that the way people wanna make a car file is "put as large a subtree as you can into each one"
// TODO eventually we're gonna start at the leftmost node. we're gonna try and get as many of its siblings as we can. if we succeed we're gonna go up a node. and get as many of its siblings as we can.

use std::path::{Path, PathBuf};
use crate::types::pipeline::{DataProcess, Pipeline};
use crate::types::shared::DataProcessDirective;
use iroh_unixfs::{
    builder::{DirectoryBuilder, FileBuilder, UnixfsEntry},
    chunker::{DEFAULT_CHUNKS_SIZE, ChunkerConfig},
};
use anyhow::Result;
use iroh_unixfs::builder::{Config, Directory, Entry};
use iroh_unixfs::chunker::ChunkerConfig::Fixed;
use iroh_unixfs::codecs::Codec::Cidv1;
use tokio_stream::StreamExt;
use crate::types::shared::DataProcessDirective::File;

// for now we're just trying to get as much of each file as we can, in order, into each car.
async fn get_in_the_car_shinji(cars_output_path: PathBuf, mut incoming_pipeline: Box<dyn Iterator<Item = Pipeline>>) -> Result<Cid>{
    // this sucks, to be clear
    // this uses iroh-unixfs over the directory where we put all the files
    // and then it carfiles them. seriously. this is so ass. please help it out
    while let Some(pipeline) = incoming_pipeline.next() {
        if let File(data_process) = pipeline.data_processing {
            for chunk_location in data_process.writeout.chunk_locations {
                let bricks_and_blocks = FileBuilder::new().path(chunk_location).fixed_chunker(DEFAULT_CHUNKS_SIZE).build().await?;
                let encoded_blockstream =

            }
        }
    }
    );
    incoming_pipeline.then(|block| ).fold(Cidv1, |acc, x| x).await;
    Ok(dir.cid)
}
