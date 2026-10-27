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
const TICK: u8 = 4;
const COMMAND: u8 = 5;
const REGISTER_COMMAND: u8 = 6;
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
    commands: Vec<String>,
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
        Some(&REGISTER_COMMAND) => inbox.commands.push(String::from_utf8(msg[1..].to_vec())?),
        Some(&REPLY) if msg.len() >= 9 => {
            let seq = u64::from_le_bytes(msg[1..9].try_into()?);
            inbox.replies.push((seq, String::from_utf8(msg[9..].to_vec())?));
        }
        other => return Err(format!("mensagem do host desconhecida: tipo {other:?}, {} bytes", msg.len()).into()),
    }
    Ok(())
}

/// Cabeçalho de um evento do host: tipo e `seq` (número do anel).
fn header(kind: u8, seq: u64) -> Vec<u8> {
    let mut m = vec![kind];
    m.extend(seq.to_le_bytes());
    m
}

/// Evento de quebra de bloco `seq` em `pos`.
fn break_event(seq: u64, pos: [i32; 3], block: &str) -> Vec<u8> {
    let mut m = header(BLOCK_BREAK, seq);
    for coord in pos {
        m.extend(coord.to_le_bytes());
    }
    m.extend(block.as_bytes());
    m
}

/// Fim de tick `tick`.
fn tick_event(seq: u64, tick: u64) -> Vec<u8> {
    let mut m = header(TICK, seq);
    m.extend(tick.to_le_bytes());
    m
}

/// Comando digitado (sem a barra).
fn command_event(seq: u64, line: &str) -> Vec<u8> {
    let mut m = header(COMMAND, seq);
    m.extend(line.as_bytes());
    m
}

struct Session<'a> {
    a: Ring<'a>,
    b: Ring<'a>,
    shared: &'a Shared,
    tail_a: u64,
    head_b: u64,
    next_seq: u64,
    inbox: Inbox,
}

impl<'a> Session<'a> {
    fn drain(&mut self) -> Res<usize> {
        let inbox = &mut self.inbox;
        self.b.drain(&mut self.head_b, |msg| parse(msg, inbox))
    }

    /// Envia um evento e acorda o Java se ele dorme (adendo v1.1).
    fn send(&mut self, side: &mut Side, msg: &[u8]) -> Res<()> {
        let tail = self.tail_a;
        wait_until("espaço no anel A", || {
            self.drain()?;
            Ok(self.a.fits(tail, msg.len()))
        })?;
        self.a.push(&mut self.tail_a, msg)?;
        self.a.publish_tail(self.tail_a);
        fence(SeqCst);
        if self.shared.sleeping_flag().load(SeqCst) == 1 {
            side.wake()?;
        }
        Ok(())
    }

    /// Manda o evento (`build` recebe o número do anel) e espera a resposta dele. Devolve o texto do mod.
    fn event(&mut self, side: &mut Side, build: impl FnOnce(u64) -> Vec<u8>) -> Res<String> {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.send(side, &build(seq))?;
        wait_until("resposta do mod", || {
            self.drain()?;
            Ok(self.inbox.replies.len() as u64 > seq)
        })?;
        let (got, text) = &self.inbox.replies[seq as usize];
        if *got != seq {
            return Err(format!("resposta fora de ordem: veio {got}, esperado {seq}").into());
        }
        Ok(text.clone())
    }

    /// Uma viagem do bench: evento `seq` até a resposta conferida. Devolve µs.
    fn trip(&mut self, side: &mut Side, seq: u64) -> Res<f64> {
        let start = Instant::now();
        let text = self.event(side, |n| break_event(n, [seq as i32, 64, -(seq as i32)], BLOCK))?;
        let elapsed = micros(start.elapsed());
        let (got, _) = &self.inbox.replies[seq as usize];
        let want = format!("gem {} {BLOCK} 64", seq + 1);
        if *got != seq || text != want {
            return Err(format!("resposta {seq}: veio ({got}, {text:?}), esperado {want:?}").into());
        }
        Ok(elapsed)
    }
}

/// Espera o sidecar ficar pronto e lê os registros que o mod mandou.
fn open<'a>(side: &mut Side, shared: &'a Shared) -> Res<(Session<'a>, Vec<ItemReg>)> {
    side.await_ready(shared)?;
    let mut s = Session { a: shared.ring_a(), b: shared.ring_b(), shared, tail_a: 0, head_b: 0, next_seq: 0, inbox: Inbox::default() };
    s.drain()?;
    let items = std::mem::take(&mut s.inbox.items);
    Ok((s, items))
}

/// Ponte de linhas para o jogo (stdin/stdout). Saída: `I <id> <json>` por item, `K <palavra>` por comando, `READY`.
/// Entrada: `B <seq> <x> <y> <z> <bloco>`, `T <seq> <tick>` ou `C <seq> <linha>`; saída: `R <seq> <mensagens>`
/// (quebra de linha vira U+001F). EOF em stdin encerra.
pub fn bridge(side: &mut Side, shared: &Shared) -> Res<()> {
    use std::io::{BufRead, Write};
    let (mut s, items) = open(side, shared)?;
    let mut out = std::io::stdout().lock();
    for i in &items {
        writeln!(out, "I {} {}", i.id, i.json)?;
    }
    for name in std::mem::take(&mut s.inbox.commands) {
        writeln!(out, "K {name}")?;
    }
    writeln!(out, "READY")?;
    out.flush()?;
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let Some((kind, rest)) = line.split_once(' ') else {
            return Err(format!("linha da ponte inválida: {line:?}").into());
        };
        let (seq, arg) = rest.split_once(' ').ok_or_else(|| format!("linha da ponte inválida: {line:?}"))?;
        let text = match kind {
            "B" => {
                let f: Vec<&str> = arg.splitn(4, ' ').collect();
                let [x, y, z, block] = f[..] else {
                    return Err(format!("linha da ponte inválida: {line:?}").into());
                };
                let pos = [x.parse()?, y.parse()?, z.parse()?];
                s.event(side, |n| break_event(n, pos, block))?
            }
            "T" => {
                let tick = arg.parse()?;
                s.event(side, |n| tick_event(n, tick))?
            }
            "C" => s.event(side, |n| command_event(n, arg))?,
            _ => return Err(format!("linha da ponte inválida: {line:?}").into()),
        };
        writeln!(out, "R {seq} {}", text.replace('\n', "\u{1f}"))?;
        out.flush()?;
    }
    Ok(())
}

/// Sobe o sidecar com o mod, lê os registros e mede `iters` eventos espaçados de `gap` (simula o tick).
pub fn run(side: &mut Side, shared: &Shared, iters: usize, gap: Duration) -> Res<HostResult> {
    let (mut s, items) = open(side, shared)?;
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

