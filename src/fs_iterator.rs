use futures::pin_mut;
use futures_util::StreamExt;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_stream::{wrappers::ReadDirStream, Stream};

#[derive(Debug)]
struct SendingToHell(Sender<(PathBuf, Option<Box<SendingToHell>>)>);
#[derive(Debug)]
struct ReceivingFromHell(Receiver<(PathBuf, Option<Box<SendingToHell>>)>);

struct FilesystemIterator {
    root_path: PathBuf,
    stack: ReceivingFromHell,
    follow_symlinks: bool,
}

impl FilesystemIterator {
    async fn new(root_path: PathBuf, follow_symlinks: bool) -> Self {
        let (tx, rx)  = tokio::sync::mpsc::channel(100); // TODO tweak 'em cowboy
        let (txh, rxh) = (SendingToHell(tx), ReceivingFromHell(rx));
        if root_path.is_dir() {
            txh.0.send((root_path.clone(), Some(Box::new(SendingToHell(txh.0.clone()))))).await.unwrap();
        } else {
            txh.0.send((root_path.clone(), None)).await.unwrap();
        }
        FilesystemIterator {
            root_path,
            stack: rxh,
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
        this.stack.0.poll_recv(cx).map(|maybe_path| {
            maybe_path.map(|(path, maybe_sender)| {
                if path.is_dir() {
                    assert!(maybe_sender.is_some());
                    let sender = maybe_sender.unwrap();
                    let path = path.clone();
                    let follow_symlinks = this.follow_symlinks;
                    // spawn a task to read the dir into the stack
                    tokio::spawn(async move {
                        // TODO: unwrapping a lot here because, like, whatever, if you can't read a dir you're probably in trouble and should start over
                        let read_dir_stream =
                            ReadDirStream::new(tokio::fs::read_dir(path).await.unwrap());
                        pin_mut!(read_dir_stream);
                        while let Some(Ok(entry)) = read_dir_stream.next().await {
                            let path = entry.path();
                            if follow_symlinks || !path.is_symlink() {
                                if entry.file_type().await.unwrap().is_dir() {
                                    sender.0.send((path, Some(Box::new(SendingToHell(sender.0.clone()))))).await.unwrap();
                                } else {
                                    sender.0.send((path, None)).await.unwrap();
                                }
                            }
                        };
                        // implicitly: sender.0.drop()
                    });
                }
                path
            })
        })
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use tokio_stream::StreamExt;
    use crate::fs_iterator::FilesystemIterator;

    // this comment lies in memoriam of the time i set these both to 10. if you estimate the disk
    // space used by a directory as only 512 bits, this would have filled 5 terabytes of disk space.
    // i'm not sure what i was thinking.
    fn make_big_filesystem_clusterfuck(depth_to_go: usize, width: usize, cwd: PathBuf) {
        if depth_to_go == 0 {
            for i in 0..width {
                let mut path = cwd.clone();
                path.push(format!("file{i}"));
                std::fs::File::create(path).unwrap();
            }
        } else {
            for i in 0..width {
                let mut path = cwd.clone();
                path.push(format!("dir{i}"));
                std::fs::create_dir(path.clone()).unwrap();
                make_big_filesystem_clusterfuck(depth_to_go - 1, width, path);
            }
        }
    }

    #[tokio::test]
    async fn run_basic_test_singlethreaded() {
        // make temp dir
        let dir = tempfile::tempdir().unwrap();
        make_big_filesystem_clusterfuck(1, 2, dir.path().to_path_buf());
        let mut iter = FilesystemIterator::new(dir.path().to_path_buf(), false).await;
        let mut count = 0;
        while let Some(file) = iter.next().await {
            println!("file: {file:?}");
            count += 1;
        }
        println!("count: {count}");
        assert_eq!(count, 7);
    }

    #[tokio::test]
    async fn it_follows_symlinks_when_told() {
        assert!(false)
    }

    #[tokio::test]
    async fn it_leaves_symlinks_when_told() {
        assert!(false)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn run_basic_test_multithreaded() {
        // make temp dir
        let dir = tempfile::tempdir().unwrap();
        make_big_filesystem_clusterfuck(1, 2, dir.path().to_path_buf());
        let mut iter = FilesystemIterator::new(dir.path().to_path_buf(), false).await;
        let mut count = 0;
        while let Some(file) = iter.next().await {
            println!("file: {file:?}");
            count += 1;
        }
        println!("count: {count}");
        assert_eq!(count, 7);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn run_bigger_test_multithreaded() {
        // make temp dir
        let dir = tempfile::tempdir().unwrap();
        make_big_filesystem_clusterfuck(5, 5, dir.path().to_path_buf());
        let mut iter = FilesystemIterator::new(dir.path().to_path_buf(), false).await;
        let mut count = 0;
        while let Some(file) = iter.next().await {
            println!("file: {file:?}");
            count += 1;
        }
        println!("count: {count}");
        assert_eq!(count, 19531);
    }


}
