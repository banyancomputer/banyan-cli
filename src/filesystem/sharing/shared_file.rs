use std::rc::Rc;
use wnfs::private::{share::SharePayload, PrivateForest};

pub struct SharedFile {
    pub payload: SharePayload,
    pub forest: Rc<PrivateForest>,
    pub file_name: String,
    pub mime_type: String,
    pub size: u64,
}
