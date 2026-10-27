//! Bench P3: latência do sidecar JVM sobre anel de memória, com linha de base por pipes.
mod args;
mod bench;
mod host;
mod msg;
mod pipe;
mod report;
mod shm;
mod sidecar;

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crate::args::Args;
use crate::bench::{BATCHES, GAP_BATCHES, Link, Plan, Row};
use crate::report::Results;
use crate::shm::Shared;
use crate::sidecar::{Side, Startup};

pub type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ERRO: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Res<()> {
    let args = args::parse(std::env::args().skip(1))?;
    let file = std::env::temp_dir().join(format!("p3-ring-{}.bin", std::process::id()));
    let shared = Shared::create(&file, shm::CAP)?;
    let text = match &args.host_jar {
        Some(jar) => run_host(&args, &shared, &file, jar),
        None => measure_all(&args, &shared, &file).map(|r| report::render(&r)),
    };
    drop(shared);
    let _ = std::fs::remove_file(&file);
    let text = text?;
    report::emit(&text, args.out.as_deref())
}

fn plan_of(args: &Args) -> Plan {
    match args.gap_us {
        Some(gap) => Plan { iters: args.iters, gap: Duration::from_micros(gap), batches: &GAP_BATCHES },
        None => Plan { iters: args.iters, gap: Duration::ZERO, batches: &BATCHES },
    }
}

fn measure_all(args: &Args, shared: &Shared, file: &Path) -> Res<Results> {
    let plan = plan_of(args);
    let java_version = report::java_version(&args.java)?;
    let (startup, rows) = bench_sidecar(args, shared, file, &plan)?;
    let pipe_rows = args
        .pipe_cp
        .as_deref()
        .map(|cp| pipe::measure_pipe(&args.java, cp, &plan))
        .transpose()?;
    let restart = crash_test(args, shared, file)?;
    let gap_us = args.gap_us;
    Ok(Results { gap_us, java_version, startup, rows, pipe_rows, restart })
}

fn bench_sidecar(args: &Args, shared: &Shared, file: &Path, plan: &Plan) -> Res<(Startup, Vec<Row>)> {
    let (mut side, mut link, startup) = start(args, shared, file)?;
    let rows = bench::measure_batches(&mut link, &mut side, plan)?;
    drop(link);
    side.stop(shared)?;
    Ok((startup, rows))
}

/// Reinicia o anel, lança o filho, espera `java_ready` e faz uma viagem de eco.
/// O Link devolvido guarda os cursores: quem o usa precisa continuar a partir deles.
fn start<'a>(args: &Args, shared: &'a Shared, file: &Path) -> Res<(Side, Link<'a>, Startup)> {
    shared.reset();
    let t0 = Instant::now();
    let mut side = Side::spawn(&args.java, &args.sidecar_cp, file, None)?;
    side.await_ready(shared)?;
    let ready = t0.elapsed();
    let mut link = Link::new(shared);
    link.trip(&mut side, 1)?;
    Ok((side, link, Startup { ready, echo: t0.elapsed() }))
}

/// Mata um filho no meio, confirma que o pai segue vivo e que um filho novo responde.
fn crash_test(args: &Args, shared: &Shared, file: &Path) -> Res<Duration> {
    let (side, _link, _) = start(args, shared, file)?;
    side.kill()?;
    let (side, _link, startup) = start(args, shared, file)?;
    side.stop(shared)?;
    Ok(startup.echo)
}

/// Mod Java vivo: registros pelo anel B e eventos de quebra de bloco medidos (intervalo padrão 1 ms).
fn run_host(args: &Args, shared: &Shared, file: &Path, jar: &Path) -> Res<String> {
    shared.reset();
    let java_version = report::java_version(&args.java)?;
    let gap = Duration::from_micros(args.gap_us.unwrap_or(1000));
    let mut side = Side::spawn(&args.java, &args.sidecar_cp, file, Some(jar))?;
    let result = host::run(&mut side, shared, args.iters, gap)?;
    host::check_gem(&result.items)?;
    host::finish(side, shared)?;
    Ok(host::render(&result, gap, &java_version))
}
