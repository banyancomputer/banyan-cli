#![feature(async_fn_in_trait)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use anyhow::Result;
use wnfs::{BlockStore, ipld::Cid};
use tokio::io::{AsyncWrite, AsyncSeek};
use fdlock::RWLock;

// TODO look at me sleepyhead https://github.com/application-research/estuary/blob/v0.4.0/util/dagsplit/dagsplitter.go

struct CarFileUnderConstruction<W: AsyncWrite + AsyncSeek> {
    /// an index to offsets and lengths of blocks inside this carfile, because we are working with a partial filesystem
    block_index: HashMap<Cid, (u64, u64)>,
    /// a list of CIDs that we're accruing as we write
    cids: Vec<Cid>,
    /// the file we're writing to
    file: RwLock<tokio::fs::File>,
}

impl BlockStore for CarFileUnderConstruction {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<Vec<u8>>> {
        if let Some((offset, length)) = self.block_index.get(cid) {
            let mut file = self.file.write().await;
            file.seek(SeekFrom::Start(*offset)).await?;
            let mut buf = vec![0; *length];
            file.read_exact(&mut buf).await?;
            // reset the file pointer
            file.seek(SeekFrom::End(0)).await?;
            Ok(Cow::Owned(buf))
        } else {
            bail!("CID not found in carfile")
        }
    }


    async fn put_block(&mut self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        assert!(bytes.len() < MAX_BLOCK_SIZE);
        let mut file = self.file.write().await;
        // make sure we're not going to get over the maximum file size now.
        // get the file size
        if file.seek(SeekFrom::End(0)).await? + bytes.len() as u64 > MAX_FILE_SIZE {
            // TODO close self and make a new file
        }
        let offset = file.seek(SeekFrom::End(0)).await?;

    }
}

enum CarFileLocation {
    UnderConstruction(CarFileUnderConstruction),
    OnDisk(PathBuf),
    //UnderDeal(Deal), // TODO this is for when it's uploaded to Filecoin :D
}

/// a blockstore of carfiles
struct CarFilesBlockStore<W: AsyncWrite + AsyncSeek> {
    /// the index maps where a given CID is to a PieceCID of a CAR file
    index: HashMap<Cid, RefCell<CarFileLocation>>,
    /// the current carfile under construction, TODO change me to a sane type
    current_carfile: RefCell<CarFileLocation>,
}

// TODO mutexin
impl<W: AsyncWrite + AsyncSeek> BlockStore for CarFilesBlockStore<W> {
    async fn get_block<'a>(&'a self, cid: &Cid) -> Result<Cow<'a, Vec<u8>>> {
        //
    }

    async fn put_block(&mut self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        assert!(bytes.len() < MAX_BLOCK_SIZE);

    }
}
