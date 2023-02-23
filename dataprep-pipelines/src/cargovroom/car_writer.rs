// copied and modified from iroh's CarWriter so i could figure out where in the file we are

use anyhow::Result;
use cid::Cid;
use tokio::io::{AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use unsigned_varint::encode as varint_encode;

use super::car_header::CarHeader;

#[derive(Debug)]
pub struct CarWriter<W: AsyncWrite + AsyncSeek + Send + Unpin> {
    header: CarHeader,
    writer: W,
    cid_buffer: Vec<u8>,
    is_header_written: bool,
}

impl<W> CarWriter<W>
where
    W: AsyncWrite + AsyncSeek + Send + Unpin,
{
    pub fn new(header: CarHeader, writer: W) -> Self {
        CarWriter {
            header,
            writer,
            cid_buffer: Vec::new(),
            is_header_written: false,
        }
    }

    /// Writes header and stream of data to writer in Car format.
    pub async fn write<T>(&mut self, cid: Cid, data: T) -> Result<()>
    where
        T: AsRef<[u8]>,
    {
        if !self.is_header_written {
            // Write header bytes
            self.writer.write_all(self.header.encode()?.as_ref()).await?;
            self.is_header_written = true;
        }

        // Write the given block.
        self.cid_buffer.clear();
        cid.write_bytes(&mut self.cid_buffer).expect("vec write");

        let data = data.as_ref();
        let len = self.cid_buffer.len() + data.len();

        let mut varint_buf = varint_encode::u64_buffer();
        let varint = varint_encode::u64((len) as u64, &mut varint_buf);
        self.writer.write_all(varint).await?;
        println!("varint: {:?}", varint);
        self.writer.write_all(&self.cid_buffer).await?;
        println!("cid: {:?}", self.cid_buffer.to_ascii_lowercase());
        println!("cid string: {:?}", cid.to_string());
        self.writer.write_all(data).await?;

        Ok(())
    }

    pub async fn underlying_location(&mut self) -> Result<u64> {
        if !self.is_header_written {
            // Write header bytes
            self.writer.write_all(self.header.encode()?.as_ref()).await?;
            self.is_header_written = true;
        }
        Ok(self.writer.seek(std::io::SeekFrom::Current(0)).await?)
    }

    /// Finishes writing, including flushing and returns the writer.
    pub async fn _finish(mut self) -> Result<W> {
        self._flush().await?;
        Ok(self.writer)
    }

    /// Flushes the underlying writer.
    pub async fn _flush(&mut self) -> Result<()> {
        self.writer.flush().await?;
        Ok(())
    }

    /// Consumes the [`CarWriter`] and returns the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer
    }
}
