//! Parse manual dos argumentos (sem clap).
use std::path::PathBuf;

use crate::Res;

pub struct Args {
    pub java: PathBuf,
    pub sidecar_cp: PathBuf,
    pub pipe_cp: Option<PathBuf>,
    pub iters: usize,
    /// Intervalo entre viagens em µs; presente = modo gap (lotes 1..1000).
    pub gap_us: Option<u64>,
    pub out: Option<PathBuf>,
    /// Presente = modo host: mod Java vivo sobre o anel em vez do bench de eco.
    pub host_jar: Option<PathBuf>,
}

pub fn parse(mut argv: impl Iterator<Item = String>) -> Res<Args> {
    let mut java = None;
    let mut sidecar_cp = None;
    let mut pipe_cp = None;
    let mut iters = 20_000;
    let mut gap_us = None;
    let mut out = None;
    let mut host_jar = None;
    while let Some(flag) = argv.next() {
        let value = argv.next().ok_or_else(|| format!("{flag} sem valor"))?;
        match flag.as_str() {
            "--java" => java = Some(PathBuf::from(value)),
            "--sidecar-cp" => sidecar_cp = Some(PathBuf::from(value)),
            "--pipe-cp" => pipe_cp = Some(PathBuf::from(value)),
            "--iters" => iters = value.parse()?,
            "--gap-us" => gap_us = Some(value.parse()?),
            "--host-jar" => host_jar = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            other => return Err(format!("argumento desconhecido: {other}").into()),
        }
    }
    if iters == 0 {
        return Err("--iters precisa ser >= 1".into());
    }
    Ok(Args {
        java: java.ok_or("falta --java")?,
        sidecar_cp: sidecar_cp.ok_or("falta --sidecar-cp")?,
        pipe_cp,
        iters,
        gap_us,
        out,
        host_jar,
    })
}
