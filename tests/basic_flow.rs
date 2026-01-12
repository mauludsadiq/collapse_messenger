use std::rc::Rc;
use std::cell::RefCell;

use collapse_messenger::node::NodeMessenger;
use collapse_messenger::types::{PubKey, Digest, zero_digest};
use collapse_messenger::phi::Evidence;
use collapse_messenger::transport_mem::MemoryTransport;
use collapse_messenger::content::Content;

#[test]
fn basic_flow_demo() {
    // shared in-memory bus
    let bus = Rc::new(RefCell::new(MemoryTransport::new()));

    // three nodes registered on the same bus
    let mut a = NodeMessenger::new(PubKey("A".into()), bus.clone());
    let mut b = NodeMessenger::new(PubKey("B".into()), bus.clone());
    let mut c = NodeMessenger::new(PubKey("C".into()), bus.clone());

    // note: we record "peers" for social knowledge; delivery is via bus.broadcast()
    a.add_peer(PubKey("B".into()));
    a.add_peer(PubKey("C".into()));
    b.add_peer(PubKey("A".into()));
    b.add_peer(PubKey("C".into()));
    c.add_peer(PubKey("A".into()));
    c.add_peer(PubKey("B".into()));

    // 1. A sends a root canonical text message (parent = zero_digest)
    let root_parent = zero_digest();
    a.send(
        root_parent,
        Evidence::DraftText { raw: "hello    world   from A".into() }
    );

    // At this instant:
    //  - A accepted its own message
    //  - bus queued copies for B and C
    // So deliver them:
    b.poll();
    c.poll();

    // Get the digest of A's last message to use as the parent for replies
    let root_digest = {
        a.inbox.last().unwrap().digest.clone()
    };

    // 2. B replies with a retinal witness message to A's message
    b.send(
        root_digest.clone(),
        Evidence::RawRetinaCapture {
            samples: vec![(0.5,0.5,0.9)], // stub sample
            lambda: 0.1,
            foveation_cfg: (0.15, 0.5, 0.5),
            basis_cfg: (32, 32),
            cert_seed: 42,
        }
    );

    // Now bus has B's retinal message queued for A and C
    a.poll();
    c.poll();

    // 3. C attempts to send an orphan reply to a digest nobody has
    let bogus_parent = Digest([9u8;32]);
    c.send(
        bogus_parent,
        Evidence::DraftText { raw: "malicious fork attempt".into() }
    );

    // That bogus message got broadcast too. Let others poll:
    a.poll();
    b.poll();

    // After that:
    // - A should have accepted legit messages
    // - A should have punished C for orphan injection
    // - Retinal content from B should be in A's inbox

    println!("A inbox len         = {}", a.inbox.len());
    println!("A rep(A)            = {}", a.rep.get(&PubKey("A".into())));
    println!("A rep(B)            = {}", a.rep.get(&PubKey("B".into())));
    println!("A rep(C)            = {}", a.rep.get(&PubKey("C".into())));

    assert!(a.inbox.len() >= 2, "A should have at least its own text + B's retinal");

    // B should have been rewarded for good behavior
    assert!(a.rep.get(&PubKey("B".into())) >= 0.6, "B should be rewarded");

    // C should have been punished for orphan injection
    assert!(a.rep.get(&PubKey("C".into())) <= 0.5, "C should be punished/quarantined");

    // sanity: A saw retinal content
    let a_saw_retina = a.inbox.iter().any(|m|
        matches!(m.content, Content::Retina(_))
    );
    println!("A saw retinal content? {}", a_saw_retina);
    assert!(a_saw_retina, "A should have received at least one Retina message from B");
}
