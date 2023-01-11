use futures::pin_mut;
use futures_util::StreamExt;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_stream::{wrappers::ReadDirStream, Stream};

struct FilesystemIterator {
    root_path: PathBuf,
    stack: (Sender<PathBuf>, Receiver<PathBuf>),
    follow_symlinks: bool,
}

impl FilesystemIterator {
    fn new(root_path: PathBuf, follow_symlinks: bool) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(100); // TODO tweak 'em cowboy
        tx.blocking_send(root_path.clone()).unwrap();
        FilesystemIterator {
            root_path,
            stack: (tx, rx),
            follow_symlinks,
        }
    }
}

impl Stream for FilesystemIterator {
    type Item = PathBuf;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        //let new_self = Pin::<&mut FilesystemIterator>::into_inner(self);
        // what's the next thing in the stack?
        this.stack.1.poll_recv(cx).map(|maybe_path| {
            maybe_path.map(|path| {
                if path.is_dir() {
                    let sender = this.stack.0.clone();
                    let path = path.clone();
                    let follow_symlinks = this.follow_symlinks;
                    // spawn a task to read the dir into the stack
                    tokio::spawn(async move {
                        // TODO: unwrapping a lot here because, like, whatever, if you can't read a dir you're probably in trouble and should start over
                        let read_dir_stream =
                            ReadDirStream::new(tokio::fs::read_dir(path).await.unwrap());
                        pin_mut!(read_dir_stream);
                        let _ = read_dir_stream.map(|entry| {
                            let entry = entry.unwrap();
                            let child_path = entry.path();
                            if follow_symlinks || !child_path.is_symlink() {
                                sender.blocking_send(child_path).unwrap();
                            }
                        });
                    });
                }
                path
            })
        })
    }
}
