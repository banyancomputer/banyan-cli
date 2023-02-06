// TODO (laudiacay): i think i'm willing to bet that the way people wanna make a car file is "put as large a subtree as you can into each one"
// TODO eventually we're gonna start at the leftmost node. we're gonna try and get as many of its siblings as we can. if we succeed we're gonna go up a node. and get as many of its siblings as we can.

use crate::types::pipeline::{DataProcess, Pipeline};
use crate::types::shared::DataProcessDirective;
use iroh_unixfs::{
    builder::{DirectoryBuilder, FileBuilder, UnixfsEntry},
    chunker::{DEFAULT_CHUNKS_SIZE, ChunkerConfig},
};
use anyhow::Result;

// for now we're just trying to get as much of each file as we can, in order, into each car.
async fn get_in_the_car_shinji(mut incoming_pipeline: Box<dyn Iterator<Item = Pipeline>>) -> Result<Cid>{
    // this sucks, to be clear
    // this uses iroh-unixfs over the directory where we put all the files
    // and then it carfiles them. seriously. this is so ass. please help it out.
    let chunker = ChunkerConfig::Fixed(DEFAULT_CHUNKS_SIZE).into();
    // TODO turn blabla into uuid
    let dir = DirectoryBuilder::new().name("run-blabla");
    while let Some(next_pipeline) = incoming_pipeline.next() {
        let dataprocess = match next_pipeline.data_processing {
            DataProcessDirective::Duplicate(_) => continue,
            DataProcessDirective::Directory => continue,
            DataProcessDirective::Symlink => continue,
            DataProcessDirective::File(f) => f,
        };

        for output_location in dataprocess.writeout.chunk_locations.iter() {
            let file = FileBuilder::new()
                .path(output_location)
                .fixed_chunker(DEFAULT_CHUNKS_SIZE)
                .build().await?;
            dir.add_file(file);
        }
    }
    Ok(dir.cid)
}
