use serde::{Serialize, Deserialize};
use crate::types::{Digest, Timestamp};
use crate::blob::BlobBody;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Content {
    Text(TextBody),
    Retina(RetinaBody),
    Status(StatusEvent),
    Blob(BlobBody),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBody {
    pub canonical_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetinaBody {
    pub omega_id: String,
    pub basis_spec: BasisSpec,
    pub a_hat: Vec<f64>,
    pub lambda: f64,
    pub foveation: FoveationSpec,
    pub cert: CertBundle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasisSpec {
    pub nx: u32,
    pub ny: u32,
    pub basis_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoveationSpec {
    pub sigma: f64,
    pub center_x: f64,
    pub center_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertBundle {
    pub psnr_equiv_db: f64,
    pub fused_variance_drop: f64,
    pub foveation_alignment_score: f64,
    pub deterministic_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusEvent {
    Delivered { digest_ack: Digest, at: Timestamp },
    Read { digest_ack: Digest, at: Timestamp },
    TypingStart,
    TypingStop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub sender: crate::types::PubKey,
    pub parent: Digest,
    pub content: Content,
    pub digest: Digest,
    pub signature: crate::types::Signature,
    pub timestamp: Timestamp,
}
