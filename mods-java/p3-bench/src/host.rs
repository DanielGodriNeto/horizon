//! Modo `--host-jar`: um mod Java vivo no sidecar. Confere os registros que chegam pelo anel B e mede
//! eventos de quebra de bloco (Rust -> mod -> resposta). Formato das mensagens: PROTOCOLO.md, "Mensagens do host".
use std::sync::atomic::{Ordering::SeqCst, fence};
use std::thread;
use std::time::{Duration, Instant};

use crate::Res;
use crate::bench::{Row, micros};
use crate::shm::{Ring, Shared, wait_until};
use crate::sidecar::Side;

const REGISTER_ITEM: u8 = 1;
const BLOCK_BREAK: u8 = 2;
const REPLY: u8 = 3;
const BLOCK: &str = "minecraft:stone";
const WARMUP: usize = 500;

/// Registro de item vindo do mod: id e o JSON de conteúdo.
#[derive(Debug, PartialEq)]
pub struct ItemReg {
    pub id: String,
    pub json: String,
}

/// Registros e respostas recebidos, em ordem de chegada.
#[derive(Default)]
struct Inbox {
    items: Vec<ItemReg>,
    replies: Vec<(u64, String)>,
}

pub struct HostResult {
    pub items: Vec<ItemReg>,
    pub tick: Row,
}

fn parse(msg: &[u8], inbox: &mut Inbox) -> Res<()> {
    match msg.first() {
        Some(&REGISTER_ITEM) if msg.len() >= 3 => {
            let n = u16::from_le_bytes([msg[1], msg[2]]) as usize;
            let id = msg.get(3..3 + n).ok_or("REGISTER_ITEM truncado")?;
            let json = &msg[3 + n..];
            inbox.items.push(ItemReg { id: String::from_utf8(id.to_vec())?, json: String::from_utf8(json.to_vec())? });
        }
        Some(&REPLY) if msg.len() >= 9 => {
            let seq = u64::from_le_bytes(msg[1..9].try_into()?);
            inbox.replies.push((seq, String::from_utf8(msg[9..].to_vec())?));
        }
        other => return Err(format!("mensagem do host desconhecida: tipo {other:?}, {} bytes", msg.len()).into()),
    }
    Ok(())
}

fn break_event(seq: u64) -> Vec<u8> {
    let mut m = vec![BLOCK_BREAK];
    m.extend(seq.to_le_bytes());
    for coord in [seq as i32, 64, -(seq as i32)] {
        m.extend(coord.to_le_bytes());
    }
    m.extend(BLOCK.as_bytes());
    m
}

struct Session<'a> {
    a: Ring<'a>,
    b: Ring<'a>,
    shared: &'a Shared,
    tail_a: u64,
    head_b: u64,
    inbox: Inbox,
}

impl<'a> Session<'a> {
    fn drain(&mut self) -> Res<usize> {
        let inbox = &mut self.inbox;
        self.b.drain(&mut self.head_b, |msg| parse(msg, inbox))
    }

    /// Envia um evento e acorda o Java se ele dorme (adendo v1.1).
    fn send(&mut self, side: &mut Side, seq: u64) -> Res<()> {
        let msg = break_event(seq);
        let tail = self.tail_a;
        wait_until("espaço no anel A", || {
            self.drain()?;
            Ok(self.a.fits(tail, msg.len()))
        })?;
        self.a.push(&mut self.tail_a, &msg)?;
        self.a.publish_tail(self.tail_a);
        fence(SeqCst);
        if self.shared.sleeping_flag().load(SeqCst) == 1 {
            side.wake()?;
        }
        Ok(())
    }

    /// Uma viagem: evento `seq` até a resposta dele. Devolve µs.
    fn trip(&mut self, side: &mut Side, seq: u64) -> Res<f64> {
        let start = Instant::now();
        self.send(side, seq)?;
        wait_until("resposta do mod", || {
            self.drain()?;
            Ok(self.inbox.replies.len() as u64 > seq)
        })?;
        let elapsed = micros(start.elapsed());
        let (got, text) = &self.inbox.replies[seq as usize];
        let want = format!("gem {} {BLOCK} 64", seq + 1);
        if *got != seq || *text != want {
            return Err(format!("resposta {seq}: veio ({got}, {text:?}), esperado {want:?}").into());
        }
        Ok(elapsed)
    }
}

/// Sobe o sidecar com o mod, lê os registros e mede `iters` eventos espaçados de `gap` (simula o tick).
pub fn run(side: &mut Side, shared: &Shared, iters: usize, gap: Duration) -> Res<HostResult> {
    side.await_ready(shared)?;
    let mut s = Session { a: shared.ring_a(), b: shared.ring_b(), shared, tail_a: 0, head_b: 0, inbox: Inbox::default() };
    s.drain()?;
    let items = std::mem::take(&mut s.inbox.items);
    let mut samples = Vec::with_capacity(iters);
    for seq in 0..(WARMUP + iters) as u64 {
        let us = s.trip(side, seq)?;
        if seq as usize >= WARMUP {
            samples.push(us);
        }
        if !gap.is_zero() {
            thread::sleep(gap);
        }
    }
    Ok(HostResult { items, tick: Row::from_samples(1, &mut samples) })
}

/// Confere que o mod registrou `gemjava:gem` com pilha 16, pelo anel.
pub fn check_gem(items: &[ItemReg]) -> Res<()> {
    let gem = items.iter().find(|i| i.id == "gemjava:gem").ok_or("gemjava:gem não chegou pelo anel B")?;
    if !gem.json.contains("\"max_stack_size\": 16") {
        return Err(format!("gemjava:gem sem pilha 16: {}", gem.json).into());
    }
    Ok(())
}

pub fn render(r: &HostResult, gap: Duration, java_version: &str) -> String {
    let mut out = format!("# Mod Java vivo sobre o anel\n\nJava: {java_version}\n\nRegistros recebidos pelo anel B:\n");
    for i in &r.items {
        out += &format!("- `{}` {}\n", i.id, i.json);
    }
    out += &format!(
        "\nEvento de quebra de bloco (n={}, intervalo {} ms): p50 {:.1} us, p99 {:.1} us, p99.9 {:.1} us, max {:.1} us\n",
        r.tick.n,
        gap.as_millis(),
        r.tick.p50,
        r.tick.p99,
        r.tick.p999,
        r.tick.max
    );
    out
}

/// Para o filho normalmente (shutdown + 'bye').
pub fn finish(side: Side, shared: &Shared) -> Res<()> {
    side.stop(shared)
}

