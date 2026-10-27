//! Ciclo de vida do filho Sidecar (Java): lançar, esperar pronto, parar normalmente, matar.
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::Ordering::{Acquire, Release};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::Res;
use crate::shm::{Shared, wait_until};

const JVM_FLAGS: [&str; 4] = ["-XX:+UseSerialGC", "-Xmx64m", "-Xshare:auto", "-cp"];

pub struct Startup {
    pub ready: Duration,
    pub echo: Duration,
}

pub struct Side {
    child: Child,
    stdin: ChildStdin,
    stdout: Option<JoinHandle<String>>,
}

impl Side {
    /// Lança `<java> -XX:+UseSerialGC -Xmx64m -Xshare:auto -cp <cp> Sidecar <arquivo> echo`, stdin em pipe (despertar).
    pub fn spawn(java: &Path, cp: &Path, file: &Path) -> Res<Side> {
        let mut child = Command::new(java)
            .args(JVM_FLAGS)
            .arg(cp)
            .arg("Sidecar")
            .arg(file)
            .arg("echo")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().ok_or("sidecar sem stdin")?;
        let mut out = child.stdout.take().ok_or("sidecar sem stdout")?;
        let stdout = thread::spawn(move || {
            let mut text = String::new();
            let _ = out.read_to_string(&mut text);
            text
        });
        Ok(Side { child, stdin, stdout: Some(stdout) })
    }

    /// Acorda o Java se ele estiver dormindo no stdin (adendo v1.1). Bytes extras são ignorados pelo Java.
    pub fn wake(&mut self) -> Res<()> {
        self.stdin.write_all(&[1])?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Espera `java_ready == 1`; erro se o filho sair antes.
    pub fn await_ready(&mut self, shared: &Shared) -> Res<()> {
        let child = &mut self.child;
        wait_until("java_ready", || {
            if child.try_wait()?.is_some() {
                return Err("sidecar saiu antes de ficar pronto".into());
            }
            Ok(shared.java_ready().load(Acquire) == 1)
        })
    }

    /// Parada normal: `shutdown=1`, saída com status 0 e linha `bye` na saída do filho.
    pub fn stop(mut self, shared: &Shared) -> Res<()> {
        shared.shutdown_flag().store(1, Release);
        self.wake()?;
        let status = self.wait_exit()?;
        let text = self.join_stdout()?;
        if !status.success() {
            return Err(format!("sidecar saiu com {status}").into());
        }
        if !text.lines().any(|line| line.trim() == "bye") {
            return Err("sidecar saiu sem imprimir 'bye'".into());
        }
        Ok(())
    }

    /// Mata o filho à força (simula crash) e espera a reaping.
    pub fn kill(mut self) -> Res<()> {
        self.child.kill()?;
        self.child.wait()?;
        Ok(())
    }

    fn wait_exit(&mut self) -> Res<ExitStatus> {
        let child = &mut self.child;
        let mut status = None;
        wait_until("saída do sidecar", || {
            status = child.try_wait()?;
            Ok(status.is_some())
        })?;
        status.ok_or_else(|| "status de saída ausente".into())
    }

    fn join_stdout(&mut self) -> Res<String> {
        let handle = self.stdout.take().ok_or("stdout do sidecar já coletado")?;
        handle.join().map_err(|_| "leitor de stdout do sidecar entrou em pânico".into())
    }
}

impl Drop for Side {
    fn drop(&mut self) {
        // Evita Java órfão segurando o mapeamento quando o bench falha no meio.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
