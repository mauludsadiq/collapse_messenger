use std::rc::Rc;
use std::cell::RefCell;

use collapse_messenger::node::NodeMessenger;
use collapse_messenger::types::{PubKey, zero_digest, Digest};
use collapse_messenger::phi::Evidence;
use collapse_messenger::transport_mem::MemoryTransport;
use collapse_messenger::content::{Content, StatusEvent, RetinaBody};
use collapse_messenger::fuse::fuse_fixations;

#[test]
fn extended_flow_demo() {
    // shared bus
    let bus = Rc::new(RefCell::new(MemoryTransport::new()));

    // nodes
    let mut a = NodeMessenger::new(PubKey("A".into()), bus.clone());
    let mut b = NodeMessenger::new(PubKey("B".into()), bus.clone());
    let mut c = NodeMessenger::new(PubKey("C".into()), bus.clone());

    // "social" peers (not strictly required for bus broadcast,
    // but the node tracks them conceptually)
    a.add_peer(PubKey("B".into()));
    a.add_peer(PubKey("C".into()));
    b.add_peer(PubKey("A".into()));
    b.add_peer(PubKey("C".into()));
    c.add_peer(PubKey("A".into()));
    c.add_peer(PubKey("B".into()));

    // 1. A sends root text
    let root_parent = zero_digest();
    a.send(
        root_parent,
        Evidence::DraftText { raw: "THIS_IS_ROOT_MSG_FROM_A".into() }
    );

    // deliver to B and C
    b.poll();
    c.poll();

    // root digest (from A's perspective)
    let root_digest = {
        a.inbox.last().unwrap().digest.clone()
    };

    // 2. B sends TWO retinal captures (simulate two fixations), replying to root_digest
    b.send(
        root_digest.clone(),
        Evidence::RawRetinaCapture {
            samples: vec![(0.5, 0.5, 0.9), (0.6, 0.5, 0.8)],
            lambda: 0.1,
            foveation_cfg: (0.15, 0.5, 0.5),
            basis_cfg: (32, 32),
            cert_seed: 7,
        }
    );

    b.send(
        root_digest.clone(),
        Evidence::RawRetinaCapture {
            samples: vec![(0.4, 0.52, 0.92), (0.55, 0.47, 0.81)],
            lambda: 0.1,
            foveation_cfg: (0.15, 0.5, 0.5),
            basis_cfg: (32, 32),
            cert_seed: 8,
        }
    );

    // deliver those to A and C
    a.poll();
    c.poll();

    // 3. B acknowledges delivery/read of A's root
    b.ack_delivered(root_digest.clone());
    b.ack_read(root_digest.clone());

    // deliver those status events to A and C
    a.poll();
    c.poll();

    // 4. (Optional) C misbehaves with an orphan to prove rep slashing still works
    let bogus_parent = Digest([7u8;32]);
    c.send(
        bogus_parent,
        Evidence::DraftText { raw: "i am chaos".into() }
    );
    a.poll();
    b.poll();

    // ---- ASSERTIONS ON A'S VIEW ----
    {
        println!("A inbox len (extended) = {}", a.inbox.len());
        assert!(a.inbox.len() >= 5, "A should have root + 2 retina + 2 status events minimum");

        // Extract all RetinaBody messages that A has accepted
        let retina_msgs: Vec<&RetinaBody> = a.inbox.iter().filter_map(|m| {
            if let Content::Retina(ref r) = m.content {
                Some(r)
            } else {
                None
            }
        }).collect();

        println!("A sees {} retinal packets", retina_msgs.len());
        assert!(retina_msgs.len() >= 2, "A should have at least 2 retinal packets from B");

        for (idx, r) in retina_msgs.iter().enumerate() {
            println!("Retina[{}] lambda = {}", idx, r.lambda);
            println!("Retina[{}] foveation sigma = {}", idx, r.foveation.sigma);
            println!("Retina[{}] a_hat len = {}", idx, r.a_hat.len());
            assert!(r.a_hat.len() > 0, "a_hat should encode canonical capture state");
            assert!(r.cert.psnr_equiv_db >= 80.0);
            assert!(r.cert.foveation_alignment_score >= 1.0);
        }

        // confirm Delivered / Read receipts from B targeting root_digest
        let mut saw_delivered = false;
        let mut saw_read = false;
        for m in &a.inbox {
            if let Content::Status(StatusEvent::Delivered { digest_ack, .. }) = &m.content {
                if *digest_ack == root_digest {
                    saw_delivered = true;
                    println!("A sees Delivered ack from B for root digest");
                }
            }
            if let Content::Status(StatusEvent::Read { digest_ack, .. }) = &m.content {
                if *digest_ack == root_digest {
                    saw_read = true;
                    println!("A sees Read ack from B for root digest");
                }
            }
        }
        assert!(saw_delivered, "A should have a Delivered receipt from B");
        assert!(saw_read, "A should have a Read receipt from B");

        // check rep movement from A's viewpoint
        let rep_b = a.rep.get(&PubKey("B".into()));
        let rep_c = a.rep.get(&PubKey("C".into()));
        println!("A rep(B) after retina+acks = {}", rep_b);
        println!("A rep(C) after orphan      = {}", rep_c);
        assert!(rep_b >= 0.6, "B should be rewarded");
        assert!(rep_c <= 0.5, "C should be punished/quarantined");
    }

    // ---- FUSION TEST ----
    // A fuses all retinal packets it saw from B into a single canonical scene.
    {
        let retina_packets: Vec<RetinaBody> = a.inbox.iter().filter_map(|m| {
            if let Content::Retina(ref r) = m.content {
                Some(r.clone())
            } else {
                None
            }
        }).collect();

        let fused_candidate = fuse_fixations(&retina_packets);
        match fused_candidate {
            None => panic!("expected fused retina"),
            Some(fused) => {
                println!("Fused digest = {:?}", fused.fused_digest);
                println!("Fused variance_drop = {}", fused.fused.cert.fused_variance_drop);

                // We sent 2 fixations, so fused_variance_drop should be ~1/2 (0.5)
                assert!(fused.fused.cert.fused_variance_drop <= 0.5 + 1e-9);

                // fused a_hat shouldn't be empty in our design
                assert!(fused.fused.a_hat.len() > 0);
            }
        }
    }
}
