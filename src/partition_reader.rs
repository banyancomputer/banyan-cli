use anyhow::Result;
use futures::executor;
use std::io::{Read, SeekFrom};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, BufReader, ReadBuf, Take};

/// reads a file partition from a file (and nothing more)
pub struct PartitionReader<R: AsyncRead + Unpin> {
    /// the partition segment
    _segment: (u64, u64),
    /// the file to read from
    reader: R,
}

impl PartitionReader<Take<BufReader<File>>> {
    /// Creates a new PartitionReader
    pub async fn new_from_path(segment: &(u64, u64), file: PathBuf) -> Result<Self> {
        // read that file, buffer the reads
        let mut reader = BufReader::new(File::open(file).await?);
        // scooch up to the start of the segment
        reader.seek(SeekFrom::Start(segment.0)).await?;
        // and don't past the end of the segment
        let reader = BufReader::take(reader, segment.1 - segment.0);
        // and awayyy we gooo
        Ok(Self {
            _segment: *segment,
            reader,
        })
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for PartitionReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.reader).poll_read(cx, buf)
    }
}

impl<R: AsyncRead + Unpin> Read for PartitionReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        executor::block_on(self.reader.read(buf))
    }
}
