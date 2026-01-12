use crate::content::{
    Content,
    TextBody,
    RetinaBody,
    BasisSpec,
    CertBundle,
    FoveationSpec,
    StatusEvent,
    Message,
};
use crate::blob::BlobBody;
use crate::types::{PubKey, Digest, Timestamp, compute_digest, sign_digest};
use crate::store;

/// New evidence kinds that Φ can collapse into canonical Content.
#[derive(Clone, Debug)]
pub enum Evidence {
    /// Free-form text to canonicalize (trim + collapse whitespace).
    DraftText { raw: String },

    /// Synthetic/demo retinal capture (placeholder solve right now).
    RawRetinaCapture {
        samples: Vec<(f32,f32,f32)>,
        lambda: f32,
        /// (sigma, center_x, center_y)
        foveation_cfg: (f32,f32,f32),
        /// (nx, ny) basis grid sizes
        basis_cfg: (u32,u32),
        cert_seed: u64,
    },

    /// Canonical status events (delivered/read/typing) to be wrapped as Content::Status.
    StatusIntent(StatusEvent),

    /// Arbitrary binary payload (pictures, gifs, video, docs...) with MIME.
    Blob { bytes: Vec<u8>, mime: String },
}

/// Core collapse implementation.
pub fn collapse_evidence(e: Evidence) -> Content {
    match e {
        Evidence::DraftText { raw } => {
            let canonical = raw.split_whitespace().collect::<Vec<_>>().join(" ");
            Content::Text(TextBody { canonical_text: canonical })
        }

        Evidence::RawRetinaCapture { lambda, foveation_cfg, basis_cfg, .. } => {
            let (sigma, cx, cy) = foveation_cfg;
            let (nx, ny) = basis_cfg;

            // Placeholder coefficients (7-dim fused vector) for the demo pipeline.
            let a_hat: Vec<f64> = vec![0.1, 0.2, 0.05, 0.0, -0.03, 0.07, 0.12];

            // Minimal certificate bundle with deterministic placeholders.
            let cert = CertBundle {
                psnr_equiv_db: 80.0_f64,
                fused_variance_drop: 0.0_f64,
                deterministic_hash: "demo-cert".to_string(),
                foveation_alignment_score: 1.0_f64,
            };

            let retina = RetinaBody {
                lambda: lambda as f64,
                omega_id: "omega/0".to_string(),
                basis_spec: BasisSpec {
                    nx,
                    ny,
                    basis_fingerprint: "basis/demo".to_string(),
                },
                foveation: FoveationSpec {
                    sigma: sigma as f64,
                    center_x: cx as f64,
                    center_y: cy as f64,
                },
                a_hat,
                cert,
            };

            Content::Retina(retina)
        }

        Evidence::StatusIntent(evt) => {
            Content::Status(evt)
        }

        Evidence::Blob { bytes, mime } => {
            let len = bytes.len();
            let object_digest = store::put(&bytes).expect("CAS write failed");
            let body = BlobBody { mime, len, object_digest };
            Content::Blob(body)
        }
    }
}

/// Public collapse entry used by NodeMessenger.
pub fn phi_collapse(e: Evidence) -> Content {
    collapse_evidence(e)
}

/// Assemble a signed, digested message.
pub fn assemble_message(
    sender: &PubKey,
    parent: Digest,
    content: Content,
    timestamp: Timestamp,
) -> Message {
    let digest = compute_digest(&content);
    let signature = sign_digest(sender, &digest);
    Message {
        sender: sender.clone(),
        parent,
        content,
        digest,
        signature,
        timestamp,
    }
}
