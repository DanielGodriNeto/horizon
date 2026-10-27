//! Saída em markdown (stdout e --out) e identificação do ambiente.
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::Res;
use crate::bench::Row;
use crate::sidecar::Startup;

pub struct Results {
    pub gap_us: Option<u64>,
    pub java_version: String,
    pub startup: Startup,
    pub rows: Vec<Row>,
    pub pipe_rows: Option<Vec<Row>>,
    pub restart: Duration,
}

/// `java -version` (o JDK escreve em stderr), numa linha só.
pub fn java_version(java: &Path) -> Res<String> {
    let out = Command::new(java).arg("-version").output()?;
    let text = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    Ok(lines.join(" / "))
}

fn host_line() -> String {
    let threads = thread::available_parallelism().map_or(0, |n| n.get());
    format!("Windows, {threads} threads lógicas")
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

pub fn render(r: &Results) -> String {
    let mut lines = vec![
        "# P3 - latência do sidecar JVM".to_string(),
        String::new(),
        format!("- Plataforma: {}", host_line()),
        match r.gap_us {
            Some(gap) => format!("- Modo gap: intervalo de {gap} µs entre viagens (fora da medição)"),
            None => "- Modo: sem intervalo (lotes 1 a 10000)".to_string(),
        },
        format!("- Java (`java -version`): {}", r.java_version),
        format!(
            "- Spawn até java_ready: {:.1} ms; até primeiro eco: {:.1} ms",
            ms(r.startup.ready),
            ms(r.startup.echo)
        ),
        format!("RESTART OK: {:.1} ms (kill, relançado, java_ready e eco)", ms(r.restart)),
        String::new(),
        "## Sidecar (anel de memória)".to_string(),
    ];
    lines.extend(table(&r.rows));
    if let Some(pipe) = &r.pipe_rows {
        lines.push(String::new());
        lines.push("## Linha de base (PipeEcho, pipes stdin/stdout)".to_string());
        lines.extend(table(pipe));
    }
    lines.join("\n") + "\n"
}

fn table(rows: &[Row]) -> Vec<String> {
    let mut out = vec![
        "| lote k | viagens | p50 µs | p99 µs | p99.9 µs | máx µs | msgs/s |".to_string(),
        "|---:|---:|---:|---:|---:|---:|---:|".to_string(),
    ];
    out.extend(rows.iter().map(|r| {
        format!(
            "| {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.0} |",
            r.k, r.n, r.p50, r.p99, r.p999, r.max, r.msgs_per_s
        )
    }));
    out
}

pub fn emit(text: &str, out: Option<&Path>) -> Res<()> {
    print!("{text}");
    if let Some(path) = out {
        std::fs::write(path, text)?;
    }
    Ok(())
}
