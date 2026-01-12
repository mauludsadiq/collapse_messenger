use crate::content::RetinaBody;
use crate::types::{Digest, compute_digest};

#[derive(Debug, Clone)]
pub struct FusedRetina {
    pub fused: RetinaBody,
    pub fused_digest: Digest,
}

/// fuse_fixations:
/// Input: slice of RetinaBody packets from successive fixations / saccades
/// Output: canonical fused RetinaBody + its digest
///
/// TODO:
/// - Stack all a_hat from inputs
/// - Recompute cert:
///     fused_variance_drop ~ 1 / sum_j M_j
/// - Solve global fuse of coefficients
///
/// For now:
/// - if multiple captures, we pick the first but adjust cert.fused_variance_drop
///   to reflect number of fixations, so digest changes deterministically with J.
pub fn fuse_fixations(retinas: &[RetinaBody]) -> Option<FusedRetina> {
    let first = retinas.first()?.clone();
    let j = retinas.len() as f64;

    let mut fused = first.clone();
    if j > 0.0 {
        // rewrite fused_variance_drop to reflect 1 / total_fixations
        let new_drop = 1.0 / j;
        let mut new_cert = fused.cert.clone();
        new_cert.fused_variance_drop = new_drop;
        fused.cert = new_cert;
    }

    let dig = compute_digest(&fused);
    Some(FusedRetina {
        fused,
        fused_digest: dig,
    })
}
