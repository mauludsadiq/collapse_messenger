use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use collapse_messenger::blob::BlobBody;
use collapse_messenger::content::{Content, StatusEvent, Message};
use collapse_messenger::node::NodeMessenger;
use collapse_messenger::phi::Evidence;
use collapse_messenger::store;
use collapse_messenger::transport_mem::MemoryTransport;
use collapse_messenger::types::{Digest, PubKey, Timestamp, zero_digest};

struct Net {
    a: NodeMessenger,
    b: NodeMessenger,
    c: NodeMessenger,
}

impl Net {
    fn new() -> Self {
        let bus = Rc::new(RefCell::new(MemoryTransport::new()));

        let mut a = NodeMessenger::new(PubKey("A".to_string()), bus.clone());
        let mut b = NodeMessenger::new(PubKey("B".to_string()), bus.clone());
        let mut c = NodeMessenger::new(PubKey("C".to_string()), bus.clone());

        // Fully connect A, B, C as peers.
        a.add_peer(b.id.clone());
        a.add_peer(c.id.clone());

        b.add_peer(a.id.clone());
        b.add_peer(c.id.clone());

        c.add_peer(a.id.clone());
        c.add_peer(b.id.clone());

        Net { a, b, c }
    }

    fn node_mut(&mut self, who: &str) -> Option<&mut NodeMessenger> {
        match who {
            "A" => Some(&mut self.a),
            "B" => Some(&mut self.b),
            "C" => Some(&mut self.c),
            _ => None,
        }
    }

    fn node_ref(&self, who: &str) -> Option<&NodeMessenger> {
        match who {
            "A" => Some(&self.a),
            "B" => Some(&self.b),
            "C" => Some(&self.c),
            _ => None,
        }
    }

    fn last_digest(&self, who: &str) -> Option<Digest> {
        let inbox: &Vec<Message> = match who {
            "A" => &self.a.inbox,
            "B" => &self.b.inbox,
            "C" => &self.c.inbox,
            _ => return None,
        };
        inbox.last().map(|m| m.digest.clone())
    }

    fn parent_for(&self, who: &str, sel: &str) -> Option<Digest> {
        match sel {
            "root" => Some(zero_digest()),
            "last" => self.last_digest(who).or_else(|| Some(zero_digest())),
            _ => {
                eprintln!("parent selector must be root|last");
                None
            }
        }
    }

    fn cmd_send_text(&mut self, from: &str, parent_sel: &str, body: &str) {
        let parent = match self.parent_for(from, parent_sel) {
            Some(d) => d,
            None => return,
        };

        let n = match self.node_mut(from) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", from);
                return;
            }
        };

        n.send(parent, Evidence::DraftText { raw: body.to_string() });
    }

    fn cmd_send_retina(&mut self, from: &str, parent_sel: &str) {
        let parent = match self.parent_for(from, parent_sel) {
            Some(d) => d,
            None => return,
        };

        let n = match self.node_mut(from) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", from);
                return;
            }
        };

        // Minimal deterministic placeholder retina packet
        let samples = vec![(0.0_f32, 0.0_f32, 0.0_f32)];
        let lambda: f32 = 550.0;
        let foveation_cfg = (1.0_f32, 0.0_f32, 0.0_f32);
        let basis_cfg = (8_u32, 8_u32);
        let cert_seed: u64 = 0;

        n.send(
            parent,
            Evidence::RawRetinaCapture {
                samples,
                lambda,
                foveation_cfg,
                basis_cfg,
                cert_seed,
            },
        );
    }

    fn cmd_send_blob(&mut self, from: &str, parent_sel: &str, path: &str, mime: &str) {
        let parent = match self.parent_for(from, parent_sel) {
            Some(d) => d,
            None => return,
        };

        let n = match self.node_mut(from) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", from);
                return;
            }
        };

        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("read {} failed: {}", path, e);
                return;
            }
        };

        n.send(
            parent,
            Evidence::Blob {
                bytes,
                mime: mime.to_string(),
            },
        );
    }

    /// For now, send_blob_to uses the same broadcast semantics as send_blob.
    /// The `to` argument is accepted for UX symmetry but not yet used to
    /// narrow delivery, because NodeMessenger.broadcast is peer-based.
    fn cmd_send_blob_to(&mut self, from: &str, _to: &str, parent_sel: &str, path: &str, mime: &str) {
        self.cmd_send_blob(from, parent_sel, path, mime);
    }

    fn cmd_ack(&mut self, from: &str, parent_sel: &str, kind: &str) {
        let digest = match self.parent_for(from, parent_sel) {
            Some(d) => d,
            None => return,
        };

        let n = match self.node_mut(from) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", from);
                return;
            }
        };

        match kind {
            "delivered" => n.ack_delivered(digest),
            "read" => n.ack_read(digest),
            _ => eprintln!("ack kind must be delivered|read"),
        }
    }

    fn cmd_poll(&mut self, who: &str) {
        let n = match self.node_mut(who) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", who);
                return;
            }
        };
        n.poll();
    }

    fn cmd_inbox(&self, who: &str) {
        let n = match self.node_ref(who) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", who);
                return;
            }
        };

        for (i, m) in n.inbox.iter().enumerate() {
            println!("#{} {:?}", i, m.digest);
            match &m.content {
                Content::Text(t) => {
                    println!(" TEXT: {}", t.canonical_text);
                }
                Content::Retina(r) => {
                    println!(
                        " RETINA: omega_id={}, lambda={}, nx={}, ny={}",
                        r.omega_id,
                        r.lambda,
                        r.basis_spec.nx,
                        r.basis_spec.ny
                    );
                }
                Content::Status(StatusEvent::Delivered { digest_ack, at }) => {
                    println!(" STATUS: delivered {:?} at {:?}", digest_ack, at);
                }
                Content::Status(StatusEvent::Read { digest_ack, at }) => {
                    println!(" STATUS: read {:?} at {:?}", digest_ack, at);
                }
                Content::Status(StatusEvent::TypingStart) => {
                    println!(" STATUS: typing start");
                }
                Content::Status(StatusEvent::TypingStop) => {
                    println!(" STATUS: typing stop");
                }
                Content::Blob(BlobBody { mime, len, object_digest }) => {
                    println!(
                        " BLOB: mime={}, len={}, object_digest={:?}",
                        mime, len, object_digest
                    );
                }
            }
        }
    }

    fn cmd_rep(&self, who: &str) {
        let _n = match self.node_ref(who) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", who);
                return;
            }
        };
        // Stub: we avoid touching ReputationBook internals here.
        println!("rep {}: <not implemented in REPL>", who);
    }

    fn cmd_fuse(&mut self, who: &str) {
        let n = match self.node_mut(who) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", who);
                return;
            }
        };
        // For now, treat "fuse" as a heal.
        n.heal();
        println!("fuse {}: heal() applied", who);
    }

    fn cmd_open_blob(&self, who: &str, idx: usize, path: &str) {
        let n = match self.node_ref(who) {
            Some(n) => n,
            None => {
                eprintln!("no such node {}", who);
                return;
            }
        };

        let msg = match n.inbox.get(idx) {
            Some(m) => m,
            None => {
                eprintln!("no such message index {}", idx);
                return;
            }
        };

        match &msg.content {
            Content::Blob(body) => {
                match store::get(&body.object_digest) {
                    Ok(bytes) => {
                        if let Err(e) = std::fs::write(path, &bytes) {
                            eprintln!("write {} failed: {}", path, e);
                        } else {
                            println!("wrote {} bytes to {}", bytes.len(), path);
                        }
                    }
                    Err(e) => {
                        eprintln!("CAS get failed: {}", e);
                    }
                }
            }
            _ => {
                eprintln!("message {} is not a blob", idx);
            }
        }
    }
}

fn print_banner() {
    println!("Collapse Messenger REPL");
    println!("nodes: A, B, C");
    println!("commands:");
    println!("  send_text FROM root|last MESSAGE...");
    println!("  send_retina FROM root|last");
    println!("  send_blob FROM root|last PATH MIME");
    println!("  send_blob_to FROM TO root|last PATH MIME");
    println!("  ack FROM root|last delivered|read");
    println!("  poll WHO");
    println!("  inbox WHO");
    println!("  rep WHO");
    println!("  fuse WHO");
    println!("  open_blob WHO INDEX PATH");
    println!("  help");
    println!("  quit / exit");
    println!("—");
}

fn main() {
    let stdin = io::stdin();
    let mut net = Net::new();

    print_banner();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0];

        match cmd {
            "quit" | "exit" => break,

            "help" => {
                print_banner();
            }

            "send_text" => {
                if parts.len() < 4 {
                    eprintln!("usage: send_text FROM root|last MESSAGE...");
                } else {
                    let from = parts[1];
                    let parent_sel = parts[2];
                    let body = parts[3..].join(" ");
                    net.cmd_send_text(from, parent_sel, &body);
                }
            }

            "send_retina" => {
                if parts.len() != 3 {
                    eprintln!("usage: send_retina FROM root|last");
                } else {
                    let from = parts[1];
                    let parent_sel = parts[2];
                    net.cmd_send_retina(from, parent_sel);
                }
            }

            "send_blob" => {
                if parts.len() != 5 {
                    eprintln!("usage: send_blob FROM root|last PATH MIME");
                } else {
                    let from = parts[1];
                    let parent_sel = parts[2];
                    let path = parts[3];
                    let mime = parts[4];
                    net.cmd_send_blob(from, parent_sel, path, mime);
                }
            }

            "send_blob_to" => {
                if parts.len() != 6 {
                    eprintln!("usage: send_blob_to FROM TO root|last PATH MIME");
                } else {
                    let from = parts[1];
                    let to = parts[2];
                    let parent_sel = parts[3];
                    let path = parts[4];
                    let mime = parts[5];
                    net.cmd_send_blob_to(from, to, parent_sel, path, mime);
                }
            }

            "ack" => {
                if parts.len() != 4 {
                    eprintln!("usage: ack FROM root|last delivered|read");
                } else {
                    let from = parts[1];
                    let parent_sel = parts[2];
                    let kind = parts[3];
                    net.cmd_ack(from, parent_sel, kind);
                }
            }

            "poll" => {
                if parts.len() != 2 {
                    eprintln!("usage: poll WHO");
                } else {
                    let who = parts[1];
                    net.cmd_poll(who);
                }
            }

            "inbox" => {
                if parts.len() != 2 {
                    eprintln!("usage: inbox WHO");
                } else {
                    let who = parts[1];
                    net.cmd_inbox(who);
                }
            }

            "rep" => {
                if parts.len() != 2 {
                    eprintln!("usage: rep WHO");
                } else {
                    let who = parts[1];
                    net.cmd_rep(who);
                }
            }

            "fuse" => {
                if parts.len() != 2 {
                    eprintln!("usage: fuse WHO");
                } else {
                    let who = parts[1];
                    net.cmd_fuse(who);
                }
            }

            "open_blob" => {
                if parts.len() != 4 {
                    eprintln!("usage: open_blob WHO INDEX PATH");
                } else {
                    let who = parts[1];
                    let idx: usize = match parts[2].parse() {
                        Ok(i) => i,
                        Err(e) => {
                            eprintln!("bad INDEX {}: {}", parts[2], e);
                            continue;
                        }
                    };
                    let path = parts[3];
                    net.cmd_open_blob(who, idx, path);
                }
            }

            _ => {
                eprintln!("unknown command \"{}\"", cmd);
            }
        }
    }
}
