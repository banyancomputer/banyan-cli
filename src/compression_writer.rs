// use std::future::Future;
// use std::io::{Error, Write};
// use std::pin::Pin;
// use std::task::{Context, Poll};
// use flate2::write::GzEncoder;
// use futures::future::BoxFuture;
// use futures::FutureExt;
// use futures::stream::BoxStream;
// use tokio::io::AsyncWrite;
//
// struct CompressionWriter<W: AsyncWrite> {
//     compressor: GzEncoder<W>,
//     compression_output: BoxStream<'static, std::io::Result<usize>>,
// }
//
// impl<W:AsyncWrite> CompressionWriter<W > {
//     fn new(writer: W) -> Self {
//         Self {
//             compressor: GzEncoder::new(writer, flate2::Compression::default()),
//             compression_output: Box::pin(async { Ok(0) })
//         }
//     }
//
//     fn compression_future(self: Pin<&mut Self>) -> BoxFuture<'static, std::io::Result<usize>> {
//         tokio::spawn(self.compressor.flush()).boxed()
//     }
// }
//
// const MAX_BUF_SIZE : usize = 1024 * 1024; // 1MB
//
// impl<W: AsyncWrite> AsyncWrite for CompressionWriter<W> {
//
//     fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
//         let this = self.get_mut();
//         if let Err(e) = this.compressor.write(buf) {
//             return Poll::Ready(Err(e));
//         }
//         if buf.len() >= MAX_BUF_SIZE {
//             if let Err(e) = this.compressor.flush() {
//                 return Poll::Ready(Err(e));
//             }
//         }
//         this.compression_output.poll(cx)
//     }
//
//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
//         if self.compression_output.
//     }
//
//     fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
//         todo!()
//     }
// }
