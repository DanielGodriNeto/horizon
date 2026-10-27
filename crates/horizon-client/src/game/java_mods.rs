//! Java mods in the game (`foreign` feature, a [turbo] experiment): a Java mod runs in a sidecar JVM
//! behind the p3-bench bridge, and the block-break event goes to its callback and back over a
//! shared-memory ring (`mods-java/PROTOCOLO.md`). Off unless `HORIZON_JAVA_BRIDGE` names the bridge:
//! `bridge.exe|java.exe|classpath|mod.jar`. `HORIZON_JAVA_LOG` appends one round-trip line per event.
use super::Game;
use horizon_client::text::Text;
use horizon_world::registry::BlockStateId;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, channel},
    },
    time::Instant,
};

/// The bridge is tried once per process: the client's own game never starts one.
static STARTED: AtomicBool = AtomicBool::new(false);

/// A reply line, stamped on the reader thread the moment it arrived.
struct Reply {
    seq: u64,
    text: String,
    at: Instant,
}

pub(super) struct JavaBridge {
    child: Child,
    stdin: ChildStdin,
    replies: Receiver<Reply>,
    sent: HashMap<u64, Instant>,
    next: u64,
    log: Option<std::fs::File>,
}

impl JavaBridge {
    /// Starts the bridge from the environment; `None` when it is not asked for or fails to come up.
    pub(super) fn from_env() -> Option<Self> {
        let spec = std::env::var("HORIZON_JAVA_BRIDGE").ok()?;
        let [bridge, java, cp, jar] = spec.split('|').collect::<Vec<_>>()[..] else {
            eprintln!("HORIZON_JAVA_BRIDGE: expected bridge|java|classpath|jar");
            return None;
        };
        let mut child = Command::new(bridge)
            .args(["--java", java, "--sidecar-cp", cp, "--host-jar", jar, "--bridge"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| eprintln!("java bridge: {e}"))
            .ok()?;
        let stdin = child.stdin.take()?;
        let mut out = BufReader::new(child.stdout.take()?);
        let mut line = String::new();
        loop {
            line.clear();
            if out.read_line(&mut line).ok()? == 0 {
                eprintln!("java bridge: ended before READY");
                return None;
            }
            if line.trim() == "READY" {
                break;
            }
            eprintln!("java mod registered: {}", line.trim());
        }
        let (tx, replies) = channel();
        std::thread::spawn(move || {
            for line in out.lines().map_while(Result::ok) {
                let at = Instant::now();
                let Some(rest) = line.strip_prefix("R ") else { continue };
                let (seq, text) = rest.split_once(' ').unwrap_or((rest, ""));
                let Ok(seq) = seq.parse() else { continue };
                if tx.send(Reply { seq, text: text.to_owned(), at }).is_err() {
                    break;
                }
            }
        });
        let log = std::env::var("HORIZON_JAVA_LOG").ok().and_then(|p| {
            std::fs::OpenOptions::new().create(true).append(true).open(Path::new(&p)).ok()
        });
        Some(Self { child, stdin, replies, sent: HashMap::new(), next: 0, log })
    }

    fn block_break(&mut self, pos: [i32; 3], block: &str) {
        let seq = self.next;
        self.next += 1;
        self.sent.insert(seq, Instant::now());
        let line = format!("B {seq} {} {} {} {block}\n", pos[0], pos[1], pos[2]);
        if self.stdin.write_all(line.as_bytes()).and_then(|()| self.stdin.flush()).is_err() {
            self.sent.remove(&seq);
        }
    }

    /// Replies that came in since the last call: their text, and the round trip in µs to the log.
    fn drain(&mut self) -> Vec<String> {
        let mut lines = Vec::new();
        while let Ok(r) = self.replies.try_recv() {
            if let (Some(sent), Some(log)) = (self.sent.remove(&r.seq), self.log.as_mut()) {
                let us = r.at.duration_since(sent).as_secs_f64() * 1e6;
                let _ = writeln!(log, "{} {us:.1} {}", r.seq, r.text.replace('\u{1f}', "|"));
            }
            lines.extend(r.text.split('\u{1f}').filter(|l| !l.is_empty()).map(str::to_owned));
        }
        lines
    }
}

impl Drop for JavaBridge {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

impl Game<'_> {
    /// A block the player broke, to the Java mod's callback.
    pub(super) fn java_mod_block_break(&mut self, pos: [i32; 3], state: BlockStateId) {
        // Only the game that simulates the world hears breaks, and it starts the bridge on the first.
        if self.remote.is_some() {
            return;
        }
        if self.java_mods.is_none() && !STARTED.swap(true, Ordering::Relaxed) {
            self.java_mods = JavaBridge::from_env();
        }
        if self.java_mods.is_none() {
            return;
        }
        let full = self.state_string(state);
        let name = full.split('[').next().unwrap_or(&full);
        if let Some(bridge) = self.java_mods.as_mut() {
            bridge.block_break(pos, name);
        }
    }

    /// The Java mod's lines to the player, once per tick.
    pub(super) fn tick_java_mods(&mut self) {
        let Some(bridge) = self.java_mods.as_mut() else { return };
        for line in bridge.drain() {
            self.system_message(Text::literal(line));
        }
    }
}
