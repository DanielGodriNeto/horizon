//! "Modo com mods": a hidden local Fabric server for third-party Java mods (docs-nossos/mods-java).
//! Off unless `--mods-mode` is given and the warning was accepted. The pure parts (hashes, ports, commands,
//! config) are tested; `start` and `stop_global` manage the one child process.
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Bumped when the warning text changes, so an old acceptance does not cover it.
pub const WARNING_VERSION: u32 = 1;
pub const FOLDER: &str = "mods-server";
const CONFIG_FILE: &str = "horizon-mods-server.txt";
const MC_VERSION: &str = "26.3";
const FABRIC_LAUNCHER_URL: &str =
    "https://meta.fabricmc.net/v2/versions/loader/26.3/0.20.1/1.1.2/server/jar";
/// The meta service publishes no hash: the launcher jar of this loader/launcher pair is pinned here.
const FABRIC_LAUNCHER_SHA256: &str =
    "70446947655b4f8f286aba896aa4c19a255e8ca8a730636ef2a6017ae4415d95";
const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const ALLOWED_HOSTS: [&str; 4] = [
    "piston-meta.mojang.com",
    "piston-data.mojang.com",
    "launchermeta.mojang.com",
    "meta.fabricmc.net",
];
const LAUNCH_JAR: &str = "fabric-server-launch.jar";
const MARKER: &str = "-Dhorizon.modsserver=1";
const READY_TIMEOUT: Duration = Duration::from_secs(300);
const STOP_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_XMX_MB: u32 = 1024;
const XMX_RANGE: std::ops::RangeInclusive<u32> = 512..=16384;
const NO_WINDOW: u32 = 0x0800_0000;

pub const WARNING_TITLE: &str = "Modo com mods: leia antes de ativar";
pub const WARNING_LINES: [&str; 10] = [
    "Para rodar mods Java, o Horizon inicia por tras um servidor Minecraft local oculto (so neste computador, nao aparece na rede) e conecta o jogo a ele. Ao ativar, voce aceita:",
    "- Mais memoria: cerca de 0,8 GB de RAM extra (mais, com muitos mods).",
    "- Java 25 necessario. Se nao houver, o Horizon avisa como instalar (nao instala sozinho).",
    "- So a versao 26.3. So mods de servidor e mods feitos com Polymer; mods de cliente ou visuais (shaders, minimapas, interface) nao sao suportados.",
    "- Pacote de texturas: mods Polymer enviam um pacote automatico; o modo aceita sozinho, so do servidor local.",
    "- Mods que adicionam blocos ou itens sem Polymer sao recusados: voce nao consegue entrar no mundo.",
    "- Inicio mais lento: o servidor precisa ligar antes do jogo.",
    "- Mods sao codigo de terceiros e rodam com as permissoes do seu usuario. Instale so de fontes confiaveis. Os jars ficam em mods-server/mods (o Horizon so mostra o hash deles).",
    "- Na primeira vez o lancador do Fabric baixa as proprias bibliotecas (maven.fabricmc.net, Mojang), sem verificacao do Horizon.",
    "- Ao ativar voce tambem aceita o EULA do Minecraft (minecraft.net/eula) para esse servidor.",
];

static ACTIVE: Mutex<Option<ModsServer>> = Mutex::new(None);
static PORT: AtomicU16 = AtomicU16::new(0);
static PID: AtomicU32 = AtomicU32::new(0);

pub struct ModsServer {
    child: Child,
    stdin: Option<ChildStdin>,
    dir: PathBuf,
}

#[derive(Debug, PartialEq)]
pub struct Config {
    pub accepted: Option<u32>,
    pub xmx_mb: u32,
    pub java: Option<PathBuf>,
}

impl Config {
    pub fn parse(text: &str) -> Self {
        let mut config = Self {
            accepted: None,
            xmx_mb: DEFAULT_XMX_MB,
            java: None,
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "accepted" => config.accepted = value.parse().ok(),
                "xmx" => {
                    config.xmx_mb = value
                        .parse()
                        .ok()
                        .filter(|mb| XMX_RANGE.contains(mb))
                        .unwrap_or(DEFAULT_XMX_MB)
                }
                "java" if !value.is_empty() => config.java = Some(PathBuf::from(value)),
                _ => {}
            }
        }
        config
    }
    pub fn load(root: &Path) -> Self {
        Self::parse(&std::fs::read_to_string(root.join(CONFIG_FILE)).unwrap_or_default())
    }
    pub fn is_accepted(&self) -> bool {
        self.accepted == Some(WARNING_VERSION)
    }
    /// Records the acceptance, keeping the user's other keys.
    pub fn save_accepted(root: &Path) -> std::io::Result<()> {
        let path = root.join(CONFIG_FILE);
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        let mut text: String = old
            .lines()
            .filter(|l| l.split_once('=').is_none_or(|(k, _)| k.trim() != "accepted"))
            .map(|l| format!("{l}\n"))
            .collect();
        text.push_str(&format!("accepted={WARNING_VERSION}\n"));
        std::fs::write(path, text)
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Clone, Copy, Debug)]
pub enum Hash {
    Sha1,
    Sha256,
    Sha512,
}

fn digest_reader(mut reader: impl Read, hash: Hash) -> std::io::Result<String> {
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();
    let mut sha512 = Sha512::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        match hash {
            Hash::Sha1 => sha1.update(&buf[..n]),
            Hash::Sha256 => sha256.update(&buf[..n]),
            Hash::Sha512 => sha512.update(&buf[..n]),
        }
    }
    Ok(match hash {
        Hash::Sha1 => hex(&sha1.finalize()),
        Hash::Sha256 => hex(&sha256.finalize()),
        Hash::Sha512 => hex(&sha512.finalize()),
    })
}

pub fn file_hash(path: &Path, hash: Hash) -> std::io::Result<String> {
    digest_reader(std::fs::File::open(path)?, hash)
}

/// Ok only when the file's digest equals `expected` (case-insensitive hex).
pub fn verify_file(path: &Path, hash: Hash, expected: &str) -> Result<(), String> {
    let actual = file_hash(path, hash).map_err(|e| format!("{}: {e}", path.display()))?;
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(())
    } else {
        Err(format!(
            "{} does not match its expected {hash:?} ({actual})",
            path.display()
        ))
    }
}

/// A free loopback port (the tiny window before Java binds it is retried by `start`).
pub fn choose_port() -> std::io::Result<u16> {
    Ok(TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port())
}

pub fn server_properties(port: u16) -> String {
    format!(
        "server-ip=127.0.0.1\nserver-port={port}\nonline-mode=false\nwhite-list=false\nenable-rcon=false\n\
         enable-query=false\nenable-command-block=false\nmax-players=2\nview-distance=8\nspawn-protection=0\n"
    )
}

pub fn command_args(xmx_mb: u32) -> Vec<String> {
    vec![
        "-Xms256M".into(),
        format!("-Xmx{xmx_mb}M"),
        MARKER.into(),
        "-jar".into(),
        LAUNCH_JAR.into(),
        "nogui".into(),
    ]
}

/// A mod jar name the folder scan accepts: plain characters, `.jar`, no separators.
pub fn valid_jar_name(name: &str) -> bool {
    name.len() > 4
        && name.ends_with(".jar")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
        && !name.starts_with('.')
}

/// `https://` and one of the official hosts, no credentials in the authority.
pub fn allowed_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    !authority.contains('@') && ALLOWED_HOSTS.contains(&authority)
}

/// The major version in `java -version`'s first quoted string (`"25.0.4.1"`, `"1.8.0_402"`).
pub fn parse_java_major(output: &str) -> Option<u32> {
    let quoted = output.split('"').nth(1)?;
    quoted.split(['.', '-', '+', '_']).next()?.parse().ok()
}

/// Whether a server-pushed pack URL points at the local server's own game port.
pub fn pack_url_is_local(url: &str, port: u16) -> bool {
    let Some(rest) = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    authority == format!("127.0.0.1:{port}")
}

/// The server's `127.0.0.1:port` while the mods server runs, for the quiet pack pre-accept.
pub fn local_address() -> Option<String> {
    match PORT.load(Ordering::Relaxed) {
        0 => None,
        port => Some(format!("127.0.0.1:{port}")),
    }
}

pub fn is_local(address: &str) -> bool {
    local_address().is_some_and(|a| a == address)
}

/// Whether this pack URL may be accepted without asking while connected to the local server.
pub fn quiet_pack(url: &str) -> bool {
    match PORT.load(Ordering::Relaxed) {
        0 => false,
        port => pack_url_is_local(url, port),
    }
}

fn http_get(url: &str) -> Result<Box<dyn Read + Send + Sync>, String> {
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build();
    let mut url = url.to_owned();
    for _ in 0..4 {
        if !allowed_url(&url) {
            return Err(format!("blocked download address: {url}"));
        }
        let response = agent.get(&url).call().map_err(|e| format!("{url}: {e}"))?;
        if (300..400).contains(&response.status()) {
            url = response
                .header("location")
                .ok_or_else(|| format!("{url}: redirect without a location"))?
                .to_owned();
            continue;
        }
        return Ok(response.into_reader());
    }
    Err("too many redirects".into())
}

/// Downloads to `<dest>.part`, checks the digest, then renames: a bad file never gets its final name.
fn download(url: &str, dest: &Path, hash: Hash, expected: &str) -> Result<(), String> {
    let part = dest.with_extension("part");
    let result = (|| {
        let mut reader = http_get(url)?;
        let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
        std::io::copy(&mut reader, &mut file).map_err(|e| format!("{url}: {e}"))?;
        file.flush().map_err(|e| e.to_string())?;
        verify_file(&part, hash, expected)?;
        std::fs::rename(&part, dest).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

fn fetch_json(url: &str) -> Result<serde_json::Value, String> {
    serde_json::from_reader(http_get(url)?).map_err(|e| format!("{url}: {e}"))
}

/// Mojang's server jar and its SHA-1, from the official manifest.
fn mojang_server_jar() -> Result<(String, String), String> {
    let manifest = fetch_json(MANIFEST_URL)?;
    let version_url = manifest["versions"]
        .as_array()
        .and_then(|v| v.iter().find(|v| v["id"] == MC_VERSION))
        .and_then(|v| v["url"].as_str())
        .ok_or_else(|| format!("Mojang's manifest has no {MC_VERSION}"))?;
    let server = &fetch_json(version_url)?["downloads"]["server"];
    match (server["url"].as_str(), server["sha1"].as_str()) {
        (Some(url), Some(sha1)) => Ok((url.to_owned(), sha1.to_owned())),
        _ => Err("Mojang's version file has no server download".into()),
    }
}

/// Fills in `server.jar` and the Fabric launcher when missing; both are verified before they get their names.
fn ensure_jars(dir: &Path) -> Result<(), String> {
    let server = dir.join("server.jar");
    if !server.is_file() {
        let (url, sha1) = mojang_server_jar()?;
        download(&url, &server, Hash::Sha1, &sha1)?;
    }
    let launcher = dir.join(LAUNCH_JAR);
    if launcher.is_file() {
        verify_file(&launcher, Hash::Sha256, FABRIC_LAUNCHER_SHA256)?;
    } else {
        download(
            FABRIC_LAUNCHER_URL,
            &launcher,
            Hash::Sha256,
            FABRIC_LAUNCHER_SHA256,
        )?;
    }
    Ok(())
}

/// The jars in `mods/`: plain names, zip files only. Returns (name, sha512) for the log.
pub fn scan_mods(mods: &Path) -> Result<Vec<(String, String)>, String> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(mods) else {
        return Ok(found);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_ascii_lowercase().ends_with(".jar") {
            continue;
        }
        if !valid_jar_name(&name) {
            return Err(format!("mod file name not accepted: {name}"));
        }
        let mut magic = [0u8; 2];
        let is_zip = std::fs::File::open(&path)
            .and_then(|mut f| f.read_exact(&mut magic))
            .is_ok()
            && &magic == b"PK";
        if !is_zip {
            return Err(format!("{name} is not a jar file"));
        }
        let sha512 = file_hash(&path, Hash::Sha512).map_err(|e| format!("{name}: {e}"))?;
        found.push((name, sha512));
    }
    found.sort();
    Ok(found)
}

fn java_candidates(config: &Config) -> Vec<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let mut found: Vec<PathBuf> = config.java.iter().cloned().collect();
    if let Some(home) = std::env::var_os("JAVA_HOME").filter(|h| !h.is_empty()) {
        found.push(Path::new(&home).join("bin").join(exe));
    }
    if let Some(path) = std::env::var_os("PATH") {
        found.extend(std::env::split_paths(&path).map(|d| d.join(exe)));
    }
    for var in ["ProgramFiles", "LOCALAPPDATA"] {
        let Some(base) = std::env::var_os(var) else {
            continue;
        };
        for vendor in ["Java", "Eclipse Adoptium", "Microsoft", "Zulu", "Amazon Corretto"] {
            let Ok(homes) = std::fs::read_dir(Path::new(&base).join(vendor)) else {
                continue;
            };
            found.extend(homes.flatten().map(|h| h.path().join("bin").join(exe)));
        }
    }
    found
}

fn java_major(java: &Path) -> Option<u32> {
    let out = Command::new(java).arg("-version").output().ok()?;
    parse_java_major(&String::from_utf8_lossy(&out.stderr))
}

/// A Java 25 (or newer) runtime; never installs one.
pub fn find_java(config: &Config) -> Result<PathBuf, String> {
    java_candidates(config)
        .into_iter()
        .filter(|p| p.is_file())
        .find(|p| java_major(p).is_some_and(|major| major >= 25))
        .ok_or_else(|| {
            "Java 25 was not found. Install it (for example Eclipse Temurin 25), or put its java \
             path in horizon-mods-server.txt as java=<path>, then try again."
                .to_owned()
        })
}

#[cfg(windows)]
fn creation_flags(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(NO_WINDOW);
}
#[cfg(not(windows))]
fn creation_flags(_: &mut Command) {}

fn spawn_server(java: &Path, dir: &Path, config: &Config) -> Result<ModsServer, String> {
    let log = std::fs::File::create(dir.join("server.log")).map_err(|e| e.to_string())?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    let mut command = Command::new(java);
    command
        .args(command_args(config.xmx_mb))
        .current_dir(dir)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(log)
        .stderr(err);
    for var in [
        "SystemRoot", "SYSTEMDRIVE", "PATH", "TEMP", "TMP", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "HOME",
    ] {
        if let Some(value) = std::env::var_os(var) {
            command.env(var, value);
        }
    }
    creation_flags(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start Java: {e}"))?;
    PID.store(child.id(), Ordering::Relaxed);
    let _ = std::fs::write(dir.join("server.pid"), child.id().to_string());
    let stdin = child.stdin.take();
    Ok(ModsServer {
        child,
        stdin,
        dir: dir.to_owned(),
    })
}

fn log_tail(dir: &Path) -> String {
    let text = std::fs::read_to_string(dir.join("server.log")).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(6)..].join(" | ")
}

enum Wait {
    Ready,
    Cancelled,
    Died(String),
    Timeout,
}

fn wait_ready(server: &mut ModsServer, cancel: &AtomicBool) -> Wait {
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Wait::Cancelled;
        }
        let log = std::fs::read_to_string(server.dir.join("server.log")).unwrap_or_default();
        if log.contains("Done (") {
            return Wait::Ready;
        }
        if let Ok(Some(status)) = server.child.try_wait() {
            return Wait::Died(format!("{status}: {}", log_tail(&server.dir)));
        }
        if started.elapsed() > READY_TIMEOUT {
            return Wait::Timeout;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

impl ModsServer {
    /// `stop` on the console, a wait for the world to save, then a kill.
    fn shutdown(&mut self) {
        if let Some(mut stdin) = self.stdin.take() {
            let _ = stdin.write_all(b"stop\n");
        }
        let until = Instant::now() + STOP_TIMEOUT;
        while Instant::now() < until {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        // Only our own PID: a later server may already own the slot.
        let _ = PID.compare_exchange(self.child.id(), 0, Ordering::Relaxed, Ordering::Relaxed);
        let _ = std::fs::remove_file(self.dir.join("server.pid"));
    }
}

impl Drop for ModsServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Stops the running mods server, if any. Safe from the exit hook and the panic hook: when the
/// lock is taken, falls back to killing the recorded PID.
pub fn stop_global() {
    PORT.store(0, Ordering::Relaxed);
    let taken = match ACTIVE.try_lock() {
        Ok(mut active) => active.take(),
        Err(_) => {
            kill_pid(PID.load(Ordering::Relaxed));
            return;
        }
    };
    if taken.is_none() {
        // Still starting (not in ACTIVE yet): kill the child the worker just spawned.
        kill_pid(PID.swap(0, Ordering::Relaxed));
    }
    drop(taken);
}

fn kill_pid(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/F"]);
        creation_flags(&mut command);
        let _ = command.output();
    }
    #[cfg(not(windows))]
    let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
}

/// A server left over by a client that was killed hard: ended only when the PID is a java process
/// running our launcher jar and marker (PIDs get reused).
fn kill_stale(dir: &Path) {
    let Some(pid) = std::fs::read_to_string(dir.join("server.pid"))
        .ok()
        .and_then(|t| t.trim().parse::<u32>().ok())
    else {
        return;
    };
    #[cfg(windows)]
    {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-Command",
            &format!("(Get-CimInstance Win32_Process -Filter 'ProcessId={pid}').CommandLine"),
        ]);
        creation_flags(&mut command);
        let line = command
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        if line.contains(MARKER) && line.contains(LAUNCH_JAR) {
            kill_pid(pid);
        }
    }
    #[cfg(not(windows))]
    {
        let line = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let line = String::from_utf8_lossy(&line);
        if line.contains(MARKER) && line.contains(LAUNCH_JAR) {
            kill_pid(pid);
        }
    }
    let _ = std::fs::remove_file(dir.join("server.pid"));
}

fn prepare(root: &Path, config: &Config) -> Result<(PathBuf, PathBuf), String> {
    let dir = root.join(FOLDER);
    std::fs::create_dir_all(dir.join("mods")).map_err(|e| e.to_string())?;
    kill_stale(&dir);
    let java = find_java(config)?;
    ensure_jars(&dir)?;
    for (name, sha512) in scan_mods(&dir.join("mods"))? {
        log::info!("Horizon mods mode: mod {name} sha512 {sha512}");
    }
    std::fs::write(dir.join("eula.txt"), "eula=true\n").map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join("fabric-server-launcher.properties"),
        "serverJar=server.jar\n",
    )
    .map_err(|e| e.to_string())?;
    Ok((dir, java))
}

/// Starts the hidden server on a random loopback port and waits for it; returns the port.
/// `cancel` is polled while waiting. Blocks for tens of seconds: run it on a worker thread.
pub fn start(root: &Path, config: &Config, cancel: &AtomicBool) -> Result<u16, String> {
    let (dir, java) = prepare(root, config)?;
    let mut last = String::new();
    for _ in 0..2 {
        let port = choose_port().map_err(|e| e.to_string())?;
        std::fs::write(dir.join("server.properties"), server_properties(port))
            .map_err(|e| e.to_string())?;
        let mut server = spawn_server(&java, &dir, config)?;
        match wait_ready(&mut server, cancel) {
            Wait::Ready if cancel.load(Ordering::Relaxed) => return Err("cancelled".into()),
            Wait::Ready => {
                *ACTIVE.lock().unwrap_or_else(|e| e.into_inner()) = Some(server);
                PORT.store(port, Ordering::Relaxed);
                return Ok(port);
            }
            Wait::Cancelled => return Err("cancelled".into()),
            Wait::Timeout => return Err("the server did not finish starting in time".into()),
            Wait::Died(detail) => {
                let busy = detail.contains("Address already in use") || detail.contains("BindException");
                last = detail;
                if !busy {
                    break;
                }
            }
        }
    }
    Err(format!("the mods server stopped: {last}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_match_known_vectors() {
        let dir = std::env::temp_dir().join(format!("hz-mods-hash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("abc");
        std::fs::write(&file, b"abc").unwrap();
        assert!(verify_file(&file, Hash::Sha1, "a9993e364706816aba3e25717850c26c9cd0d89d").is_ok());
        assert!(
            verify_file(
                &file,
                Hash::Sha256,
                "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
            )
            .is_ok()
        );
        assert!(verify_file(&file, Hash::Sha512, &"0".repeat(128)).is_err());
        assert!(verify_file(&file, Hash::Sha1, "").is_err());
        assert!(
            file_hash(&file, Hash::Sha512)
                .unwrap()
                .starts_with("ddaf35a193617aba")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn port_is_loopback_and_nonzero() {
        assert_ne!(choose_port().unwrap(), 0);
    }

    #[test]
    fn server_only_listens_on_loopback_without_rcon() {
        let text = server_properties(40123);
        assert!(text.contains("server-ip=127.0.0.1\n"));
        assert!(text.contains("server-port=40123\n"));
        assert!(text.contains("enable-rcon=false\n"));
        assert!(text.contains("enable-query=false\n"));
    }

    #[test]
    fn command_has_heap_marker_and_no_shell() {
        let args = command_args(2048);
        assert_eq!(
            args,
            ["-Xms256M", "-Xmx2048M", MARKER, "-jar", LAUNCH_JAR, "nogui"]
        );
    }

    #[test]
    fn config_parses_and_clamps() {
        let c = Config::parse("accepted=1\nxmx=2048\njava = C:\\jdk\\bin\\java.exe\n# x\nbad\n");
        assert_eq!(c.accepted, Some(1));
        assert_eq!(c.xmx_mb, 2048);
        assert_eq!(c.java, Some(PathBuf::from("C:\\jdk\\bin\\java.exe")));
        assert_eq!(Config::parse("xmx=64").xmx_mb, DEFAULT_XMX_MB);
        assert_eq!(Config::parse("xmx=999999").xmx_mb, DEFAULT_XMX_MB);
        assert_eq!(Config::parse("xmx=abc").xmx_mb, DEFAULT_XMX_MB);
        assert!(!Config::parse("").is_accepted());
        assert!(!Config::parse("accepted=0").is_accepted());
        assert!(Config::parse(&format!("accepted={WARNING_VERSION}")).is_accepted());
    }

    #[test]
    fn acceptance_keeps_other_keys() {
        let dir = std::env::temp_dir().join(format!("hz-mods-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(CONFIG_FILE), "xmx=3000\naccepted=0\n").unwrap();
        Config::save_accepted(&dir).unwrap();
        let c = Config::load(&dir);
        assert!(c.is_accepted());
        assert_eq!(c.xmx_mb, 3000);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn jar_names() {
        assert!(valid_jar_name("lithium-fabric-0.21.jar"));
        assert!(!valid_jar_name("..\\evil.jar"));
        assert!(!valid_jar_name("a/b.jar"));
        assert!(!valid_jar_name("with space.jar"));
        assert!(!valid_jar_name(".jar"));
        assert!(!valid_jar_name("x.exe"));
    }

    #[test]
    fn download_hosts() {
        assert!(allowed_url("https://piston-data.mojang.com/v1/objects/x/server.jar"));
        assert!(allowed_url("https://meta.fabricmc.net/v2/versions"));
        assert!(!allowed_url("http://meta.fabricmc.net/v2"));
        assert!(!allowed_url("https://meta.fabricmc.net@evil.example/x"));
        assert!(!allowed_url("https://evil.example/meta.fabricmc.net"));
        assert!(!allowed_url("https://meta.fabricmc.net.evil.example/x"));
    }

    #[test]
    fn java_versions() {
        assert_eq!(
            parse_java_major("openjdk version \"25.0.4.1\" 2026-07-14\nOpenJDK Runtime"),
            Some(25)
        );
        assert_eq!(parse_java_major("java version \"1.8.0_402\""), Some(1));
        assert_eq!(parse_java_major("openjdk version \"26-ea\" 2026"), Some(26));
        assert_eq!(parse_java_major("not java"), None);
    }

    #[test]
    fn quiet_packs_only_from_the_local_game_port() {
        assert!(pack_url_is_local("http://127.0.0.1:25999/pack.zip", 25999));
        assert!(!pack_url_is_local("http://127.0.0.1:25998/pack.zip", 25999));
        assert!(!pack_url_is_local("http://evil.example/127.0.0.1:25999", 25999));
        assert!(!pack_url_is_local("http://127.0.0.1:25999@evil.example/x", 25999));
        assert!(!pack_url_is_local("ftp://127.0.0.1:25999/x", 25999));
    }

    #[test]
    fn mod_scan_refuses_bad_names_and_non_zips() {
        let dir = std::env::temp_dir().join(format!("hz-mods-scan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("good.jar"), b"PK\x03\x04rest").unwrap();
        std::fs::write(dir.join("notes.txt"), b"hi").unwrap();
        let found = scan_mods(&dir).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "good.jar");
        std::fs::write(dir.join("fake.jar"), b"MZ").unwrap();
        assert!(scan_mods(&dir).is_err());
        std::fs::remove_file(dir.join("fake.jar")).unwrap();
        std::fs::write(dir.join("bad name.jar"), b"PK").unwrap();
        assert!(scan_mods(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
