use serde::{Serialize, Deserialize};
use crate::types::Digest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobBody {
    pub mime: String,
    pub len: usize,
    pub object_digest: Digest,
}
