//! Java mods in the game (`foreign` feature, a [turbo] experiment): a Java mod runs in a sidecar JVM
//! behind the p3-bench bridge, and the block-break, end-of-tick and command events go to its callbacks
//! and back over a shared-memory ring (`mods-java/PROTOCOLO.md`). Off unless `HORIZON_JAVA_BRIDGE`
//! names the bridge: `bridge.exe|java.exe|classpath|mod.jar`. `HORIZON_JAVA_LOG` appends one
//! round-trip line per event.
use super::Game;
use horizon_client::text::Text;
use horizon_world::registry::BlockStateId;
use std::{
    collections::{HashMap, HashSet},
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

/// Events waiting for a reply before new tick events are held back.
const MAX_PENDING: usize = 8;

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
    /// Events waiting for their reply: when sent, and the connection a command answers to.
    sent: HashMap<u64, (Instant, Option<u32>)>,
    /// The command words the mod registered.
    commands: HashSet<String>,
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
        let mut commands = HashSet::new();
        let mut line = String::new();
        loop {
            line.clear();
            if out.read_line(&mut line).ok()? == 0 {
                eprintln!("java bridge: ended before READY");
                return None;
            }
            let text = line.trim();
            if text == "READY" {
                break;
            }
            match text.strip_prefix("K ") {
                Some(name) => {
                    commands.insert(name.to_owned());
                }
                None => eprintln!("java mod registered: {text}"),
            }
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
        Some(Self { child, stdin, replies, sent: HashMap::new(), commands, next: 0, log })
    }

    /// Sends one event line (`kind`, a seq, then `args`) and remembers it until its reply.
    fn send(&mut self, kind: char, args: &str, conn: Option<u32>) {
        let seq = self.next;
        self.next += 1;
        self.sent.insert(seq, (Instant::now(), conn));
        let line = format!("{kind} {seq} {args}\n");
        if self.stdin.write_all(line.as_bytes()).and_then(|()| self.stdin.flush()).is_err() {
            self.sent.remove(&seq);
        }
    }

    fn block_break(&mut self, pos: [i32; 3], block: &str) {
        self.send('B', &format!("{} {} {} {block}", pos[0], pos[1], pos[2]), None);
    }

    /// The end of tick `tick`; held back while the mod is behind, so a stuck JVM never piles events up.
    fn tick(&mut self, tick: u64) {
        if self.sent.len() < MAX_PENDING {
            self.send('T', &tick.to_string(), None);
        }
    }

    /// True when the mod registered the command word of `line`, and the line went to it.
    fn command(&mut self, line: &str, conn: Option<u32>) -> bool {
        let word = line.split(' ').next().unwrap_or_default();
        if !self.commands.contains(word) {
            return false;
        }
        self.send('C', line, conn);
        true
    }

    /// Replies that came in since the last call, as (the connection they answer or everyone, line);
    /// the round trip in µs goes to the log.
    fn drain(&mut self) -> Vec<(Option<u32>, String)> {
        let mut lines = Vec::new();
        while let Ok(r) = self.replies.try_recv() {
            let conn = match self.sent.remove(&r.seq) {
                Some((sent, conn)) => {
                    if let Some(log) = self.log.as_mut() {
                        let us = r.at.duration_since(sent).as_secs_f64() * 1e6;
                        let _ = writeln!(log, "{} {us:.1} {}", r.seq, r.text.replace('\u{1f}', "|"));
                    }
                    conn
                }
                None => None,
            };
            let texts = r.text.split('\u{1f}').filter(|l| !l.is_empty());
            lines.extend(texts.map(|l| (conn, l.to_owned())));
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
    /// The bridge, started on first need by the game that simulates the world, once per process.
    fn java_bridge(&mut self) -> Option<&mut JavaBridge> {
        if self.remote.is_some() {
            return None;
        }
        if self.java_mods.is_none() && !STARTED.swap(true, Ordering::Relaxed) {
            self.java_mods = JavaBridge::from_env();
        }
        self.java_mods.as_mut()
    }

    /// A block the player broke, to the Java mod's callback.
    pub(super) fn java_mod_block_break(&mut self, pos: [i32; 3], state: BlockStateId) {
        if self.java_bridge().is_none() {
            return;
        }
        let full = self.state_string(state);
        let name = full.split('[').next().unwrap_or(&full);
        if let Some(bridge) = self.java_mods.as_mut() {
            bridge.block_break(pos, name);
        }
    }

    /// `/line` typed by a connection's player (`None`: run locally, answered to everyone): true when a
    /// Java mod owns the command.
    pub(super) fn java_command(&mut self, conn: Option<u32>, line: &str) -> bool {
        self.java_bridge().is_some_and(|bridge| bridge.command(line, conn))
    }

    /// The tick's end to the Java mod, and what it said since the last tick to the players.
    pub(super) fn tick_java_mods(&mut self) {
        let tick = self.ticks;
        let Some(bridge) = self.java_bridge() else { return };
        bridge.tick(tick);
        for (conn, line) in bridge.drain() {
            self.tell_java_line(conn, line);
        }
    }

    /// One line of the mod's, to the connection it answers or to everyone.
    fn tell_java_line(&mut self, conn: Option<u32>, line: String) {
        let text = Text::literal(line);
        // Single player's window shows the chat of its connection to this game, not this game's own.
        let Some(host) = self.lan.as_mut() else {
            self.system_message(text);
            return;
        };
        let content = horizon_client::lan::data::component_nbt(&text.component());
        match conn {
            Some(conn) => host.hub.send_to(
                conn,
                &horizon_protocol::packets::play::Clientbound::SystemChat(
                    horizon_protocol::packets::play::SystemChat { content, overlay: false },
                ),
            ),
            None => host.hub.system_message(content),
        }
    }
}
