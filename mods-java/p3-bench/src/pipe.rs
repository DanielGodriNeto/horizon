//! Linha de base: PipeEcho do JDK pelos pipes stdin/stdout do filho (quadros u32 LE + payload).
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::Instant;

use crate::Res;
use crate::bench::{MSG_LEN, Plan, Row, iters_for, micros, sample};
use crate::msg::{Checker, fill};

const PIPE_BATCHES: [usize; 4] = [1, 10, 100, 1000];

pub struct PipeLink {
    child: Child,
    w: BufWriter<ChildStdin>,
    r: BufReader<ChildStdout>,
    sent: u64,
    check: Checker,
}

impl PipeLink {
    pub fn spawn(java: &Path, cp: &Path) -> Res<PipeLink> {
        let mut child = Command::new(java)
            .arg("-cp")
            .arg(cp)
            .arg("PipeEcho")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().ok_or("PipeEcho sem stdin")?;
        let stdout = child.stdout.take().ok_or("PipeEcho sem stdout")?;
        Ok(PipeLink {
            child,
            w: BufWriter::new(stdin),
            r: BufReader::new(stdout),
            sent: 0,
            check: Checker::new(|_| MSG_LEN),
        })
    }

    /// Uma viagem: escrita em thread própria (evita deadlock com o buffer do pipe), depois lê as respostas.
    pub fn trip(&mut self, k: usize) -> Res<f64> {
        let start = Instant::now();
        let first = self.sent;
        let PipeLink { w, r, check, sent, .. } = self;
        let (wrote, read) = thread::scope(|s| {
            let writer = s.spawn(|| write_frames(w, first, k));
            (writer.join(), read_frames(r, check, k))
        });
        wrote.map_err(|_| "escritor do pipe entrou em pânico")??;
        read?;
        *sent += k as u64;
        Ok(micros(start.elapsed()))
    }

    /// Fecha stdin (EOF) e espera o PipeEcho sair com status 0.
    pub fn finish(self) -> Res<()> {
        let PipeLink { mut child, w, r, .. } = self;
        drop(w);
        drop(r);
        let status = child.wait()?;
        if !status.success() {
            return Err(format!("PipeEcho saiu com {status}").into());
        }
        Ok(())
    }
}

fn write_frames(w: &mut BufWriter<ChildStdin>, first: u64, k: usize) -> Res<()> {
    let mut buf = [0u8; MSG_LEN];
    for seq in first..first + k as u64 {
        fill(&mut buf, seq);
        w.write_all(&(MSG_LEN as u32).to_le_bytes())?;
        w.write_all(&buf)?;
    }
    w.flush()?;
    Ok(())
}

fn read_frames(r: &mut BufReader<ChildStdout>, check: &mut Checker, k: usize) -> Res<()> {
    let mut buf = [0u8; MSG_LEN];
    for _ in 0..k {
        let mut len = [0u8; 4];
        r.read_exact(&mut len)?;
        let n = u32::from_le_bytes(len) as usize;
        if n != MSG_LEN {
            return Err(format!("quadro de {n} bytes em vez de {MSG_LEN}").into());
        }
        r.read_exact(&mut buf)?;
        check.check(&buf)?;
    }
    Ok(())
}

/// Mede a linha de base para os lotes de `PIPE_BATCHES`, com o mesmo intervalo e limites do plano.
pub fn measure_pipe(java: &Path, cp: &Path, plan: &Plan) -> Res<Vec<Row>> {
    let mut link = PipeLink::spawn(java, cp)?;
    let rows = PIPE_BATCHES
        .iter()
        .map(|&k| sample(k, iters_for(k, plan), plan.gap, |n| link.trip(n)))
        .collect::<Res<Vec<Row>>>()?;
    link.finish()?;
    Ok(rows)
}
