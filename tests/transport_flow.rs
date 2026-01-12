use std::rc::Rc;
use std::cell::RefCell;

use collapse_messenger::node::NodeMessenger;
use collapse_messenger::types::{PubKey, Digest, zero_digest};
use collapse_messenger::phi::Evidence;
use collapse_messenger::transport_mem::MemoryTransport;
use collapse_messenger::content::Content;

#[test]
fn transport_flow_demo() {
    // one shared bus
    let bus = Rc::new(RefCell::new(MemoryTransport::new()));

    // three nodes on that bus
    let mut a = NodeMessenger::new(PubKey("A".into()), bus.clone());
    let mut b = NodeMessenger::new(PubKey("B".into()), bus.clone());
    let mut c = NodeMessenger::new(PubKey("C".into()), bus.clone());

    // they "know" each other, but that's social; bus already registered them
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
        Evidence::DraftText { raw: "hi from A".into() }
    );

    // at this point, A has accepted its own msg, and bus broadcast queued copies.
    // b and c haven't polled yet, so they haven't processed it.

    b.poll();
    c.poll();

    // capture the digest of A's last accepted message (from A's own inbox)
    let root_digest = {
        a.inbox.last().unwrap().digest.clone()
    };

    // 2. B replies with retina to A's root
    b.send(
        root_digest.clone(),
        Evidence::RawRetinaCapture {
            samples: vec![(0.5,0.5,0.9)],
            lambda: 0.1,
            foveation_cfg: (0.2, 0.5, 0.5),
            basis_cfg: (16, 16),
            cert_seed: 99,
        }
    );

    // Now bus has B's retinal message queued for others.
    a.poll();
    c.poll();

    // 3. C attempts to inject an orphan with bogus parent
    let bogus_parent = Digest([7u8;32]);
    c.send(
        bogus_parent,
        Evidence::DraftText { raw: "i am chaos".into() }
    );

    // That message broadcasts out too, but it's causality-invalid.
    a.poll();
    b.poll();

    // 4. Check A's perspective
    println!("A inbox len = {}", a.inbox.len());
    assert!(a.inbox.len() >= 2, "A should have its own text + B's retina");

    let rep_a = a.rep.get(&PubKey("A".into()));
    let rep_b = a.rep.get(&PubKey("B".into()));
    let rep_c = a.rep.get(&PubKey("C".into()));
    println!("A rep(A) = {}", rep_a);
    println!("A rep(B) = {}", rep_b);
    println!("A rep(C) = {}", rep_c);

    assert!(rep_b >= 0.6, "B should be rewarded after good retinal reply");
    assert!(rep_c <= 0.5, "C should be punished after orphan send");

    // 5. sanity: does A see retinal content from B in transport mode?
    let saw_retina = a.inbox.iter().any(|m|
        matches!(m.content, Content::Retina(_))
    );
    println!("A saw retina over transport? {}", saw_retina);
    assert!(saw_retina, "RetinaBody should survive transport and still be accepted");
}
