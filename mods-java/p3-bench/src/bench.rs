//! Viagens de ida e volta sobre o anel e estatísticas por lote.
use std::sync::atomic::{AtomicU32, Ordering::SeqCst, fence};
use std::thread;
use std::time::{Duration, Instant};

use crate::Res;
use crate::msg::{Checker, fill};
use crate::shm::{Ring, Shared, wait_until};
use crate::sidecar::Side;

pub const MSG_LEN: usize = 32;
pub const BATCHES: [usize; 5] = [1, 10, 100, 1000, 10_000];
pub const GAP_BATCHES: [usize; 4] = [1, 10, 100, 1000];
const WARMUP: usize = 2000;
const BIG_BATCH: usize = 10_000;
const MAX_BIG_ITERS: usize = 2000;
const LONG_GAP: Duration = Duration::from_micros(50_000);
const MAX_LONG_GAP_ITERS: usize = 500;

/// Plano de medição: iterações pedidas, intervalo entre viagens (fora da medição) e lotes.
pub struct Plan {
    pub iters: usize,
    pub gap: Duration,
    pub batches: &'static [usize],
}

/// Estatísticas de um lote: amostras em µs e vazão em msgs/s (k / mediana).
pub struct Row {
    pub k: usize,
    pub n: usize,
    pub p50: f64,
    pub p99: f64,
    pub p999: f64,
    pub max: f64,
    pub msgs_per_s: f64,
}

impl Row {
    pub fn from_samples(k: usize, samples: &mut [f64]) -> Row {
        samples.sort_by(f64::total_cmp);
        let p50 = percentile(samples, 50.0);
        Row {
            k,
            n: samples.len(),
            p50,
            p99: percentile(samples, 99.0),
            p999: percentile(samples, 99.9),
            max: samples[samples.len() - 1],
            msgs_per_s: k as f64 / (p50 * 1e-6),
        }
    }
}

/// Percentil por posição (nearest-rank) em amostras já ordenadas e não vazias.
fn percentile(sorted: &[f64], pct: f64) -> f64 {
    let rank = (pct / 100.0 * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

pub fn micros(d: Duration) -> f64 {
    d.as_secs_f64() * 1e6
}

/// Iterações por lote. Limites: lote 10000 até 2000; gap de 50 ms ou mais até 500.
pub fn iters_for(k: usize, plan: &Plan) -> usize {
    let mut iters = plan.iters;
    if k >= BIG_BATCH {
        iters = iters.min(MAX_BIG_ITERS);
    }
    if plan.gap >= LONG_GAP {
        iters = iters.min(MAX_LONG_GAP_ITERS);
    }
    iters
}

/// Aquece (sem intervalo) e mede `trip` (µs por viagem) para o lote `k`, dormindo `gap` entre as medidas.
pub fn sample(k: usize, iters: usize, gap: Duration, mut trip: impl FnMut(usize) -> Res<f64>) -> Res<Row> {
    for _ in 0..WARMUP {
        trip(k)?;
    }
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        samples.push(trip(k)?);
        if !gap.is_zero() {
            thread::sleep(gap);
        }
    }
    Ok(Row::from_samples(k, &mut samples))
}

/// Mede todos os lotes do plano sobre um filho já pronto.
pub fn measure_batches(link: &mut Link<'_>, side: &mut Side, plan: &Plan) -> Res<Vec<Row>> {
    plan.batches
        .iter()
        .map(|&k| sample(k, iters_for(k, plan), plan.gap, |n| link.trip(side, n)))
        .collect()
}

/// Ligação com o Java pelo anel: cursores locais do produtor (A) e do consumidor (B).
pub struct Link<'a> {
    a: Ring<'a>,
    b: Ring<'a>,
    sleeping: &'a AtomicU32,
    tail_a: u64,
    head_b: u64,
    sent: u64,
    check: Checker,
}

impl<'a> Link<'a> {
    pub fn new(shared: &'a Shared) -> Link<'a> {
        Link {
            a: shared.ring_a(),
            b: shared.ring_b(),
            sleeping: shared.sleeping_flag(),
            tail_a: 0,
            head_b: 0,
            sent: 0,
            check: Checker::new(|_| MSG_LEN),
        }
    }

    /// Uma viagem: escreve `k` mensagens, publica o tail uma vez, espera as `k` respostas. Devolve µs.
    pub fn trip(&mut self, side: &mut Side, k: usize) -> Res<f64> {
        let start = Instant::now();
        self.send(side, k)?;
        self.publish(side)?;
        let target = self.sent;
        wait_until("respostas do sidecar", || {
            self.drain_b()?;
            Ok(self.check.received() == target)
        })?;
        Ok(micros(start.elapsed()))
    }

    /// Publica `tail` de A (release), fence SeqCst e acorda o Java se ele estiver dormindo (adendo v1.1).
    fn publish(&mut self, side: &mut Side) -> Res<()> {
        self.a.publish_tail(self.tail_a);
        fence(SeqCst);
        if self.sleeping.load(SeqCst) == 1 {
            side.wake()?;
        }
        Ok(())
    }

    fn send(&mut self, side: &mut Side, k: usize) -> Res<()> {
        let mut buf = [0u8; MSG_LEN];
        for _ in 0..k {
            fill(&mut buf, self.sent);
            self.wait_room(side, MSG_LEN)?;
            self.a.push(&mut self.tail_a, &buf)?;
            self.sent += 1;
        }
        Ok(())
    }

    /// Espera espaço em A. Enquanto espera, drena B (senão Java e Rust travam um ao outro).
    fn wait_room(&mut self, side: &mut Side, len: usize) -> Res<()> {
        if self.a.fits(self.tail_a, len) {
            return Ok(());
        }
        self.publish(side)?;
        let tail = self.tail_a;
        wait_until("espaço no anel A", || {
            self.drain_b()?;
            Ok(self.a.fits(tail, len))
        })
    }

    fn drain_b(&mut self) -> Res<usize> {
        self.b.drain(&mut self.head_b, |msg| self.check.check(msg))
    }
}
