//! Arquivo mapeado do protocolo P3 (mods-java/PROTOCOLO.md) e anéis SPSC sobre ele.
use std::fs::OpenOptions;
use std::hint;
use std::marker::PhantomData;
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::{Acquire, Relaxed, Release}};
use std::thread;
use std::time::{Duration, Instant};

use memmap2::{MmapMut, MmapOptions};

use crate::Res;

#[cfg(not(target_endian = "little"))]
compile_error!("o protocolo P3 exige little-endian");

pub const CAP: usize = 1 << 20;
const MAGIC: u32 = 0x484D_4A31;
const VERSION: u32 = 1;
const HDR: usize = 64;
const CTRL: usize = 128;
const OFF_MAGIC: usize = 0;
const OFF_VERSION: usize = 4;
const OFF_CAP: usize = 8;
const OFF_READY: usize = 16;
const OFF_SHUTDOWN: usize = 20;
const OFF_SLEEPING: usize = 24;
const PAD_MARK: u32 = 0xFFFF_FFFF;
const SPIN_WINDOW: Duration = Duration::from_micros(20);
const STALL: Duration = Duration::from_secs(10);

/// Tamanho do arquivo do protocolo para um anel de `cap` bytes.
pub fn file_size(cap: usize) -> usize {
    HDR + 2 * (CTRL + cap)
}

/// Bytes ocupados por um registro com payload de `len` bytes (múltiplo de 8).
pub fn rec_size(len: usize) -> usize {
    (4 + len).div_ceil(8) * 8
}

/// Mapeamento do arquivo compartilhado com o filho Java.
pub struct Shared {
    _map: MmapMut,
    base: *mut u8,
    cap: usize,
}

impl Shared {
    /// Cria (ou trunca) o arquivo, grava o cabeçalho e mapeia.
    pub fn create(path: &Path, cap: usize) -> Res<Shared> {
        let file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path)?;
        file.set_len(file_size(cap) as u64)?;
        // SAFETY: arquivo recém-criado e de uso exclusivo deste processo até o filho ser lançado; o tamanho foi fixado acima.
        let mut map = unsafe { MmapOptions::new().map_mut(&file)? };
        let base = map.as_mut_ptr();
        let shared = Shared { _map: map, base, cap };
        shared.header_u32(OFF_MAGIC).store(MAGIC, Relaxed);
        shared.header_u32(OFF_VERSION).store(VERSION, Relaxed);
        shared.header_u32(OFF_CAP).store(cap as u32, Relaxed);
        Ok(shared)
    }

    /// Zera `java_ready`, `shutdown`, `sleeping` e os cursores dos dois anéis antes de lançar um filho novo.
    pub fn reset(&self) {
        self.java_ready().store(0, Release);
        self.shutdown_flag().store(0, Release);
        self.sleeping_flag().store(0, Release);
        for ring in [self.ring_a(), self.ring_b()] {
            ring.head().store(0, Release);
            ring.tail().store(0, Release);
        }
    }

    pub fn java_ready(&self) -> &AtomicU32 {
        self.header_u32(OFF_READY)
    }

    pub fn shutdown_flag(&self) -> &AtomicU32 {
        self.header_u32(OFF_SHUTDOWN)
    }

    /// Java escreve 1 enquanto dorme no stdin (adendo v1.1); Rust lê para decidir se acorda.
    pub fn sleeping_flag(&self) -> &AtomicU32 {
        self.header_u32(OFF_SLEEPING)
    }

    /// Anel A: Rust escreve, Java lê.
    pub fn ring_a(&self) -> Ring<'_> {
        self.ring_at(HDR)
    }

    /// Anel B: Java escreve, Rust lê.
    pub fn ring_b(&self) -> Ring<'_> {
        self.ring_at(HDR + CTRL + self.cap)
    }

    fn ring_at(&self, off: usize) -> Ring<'_> {
        Ring { ctrl: self.base.wrapping_add(off), cap: self.cap, _owner: PhantomData }
    }

    fn header_u32(&self, off: usize) -> &AtomicU32 {
        // SAFETY: `off` é 0, 4, 8, 16 ou 20 (alinhado a 4) e cai dentro dos 64 bytes de cabeçalho do mapeamento.
        unsafe { &*self.base.add(off).cast::<AtomicU32>() }
    }
}

/// Um anel SPSC. `ctrl` aponta para o bloco de controle: head em +0, tail em +64, dados em +128.
pub struct Ring<'a> {
    ctrl: *mut u8,
    cap: usize,
    _owner: PhantomData<&'a Shared>,
}

// SAFETY: o ponteiro vem de um mapeamento que vive em `Shared` (emprestado por 'a). Cursores são locais
// a cada thread; o estado compartilhado é só atômico ou publicado com release/acquire.
unsafe impl Sync for Ring<'_> {}

impl Ring<'_> {
    fn head(&self) -> &AtomicU64 {
        // SAFETY: +0 do bloco de controle, alinhado a 8 bytes dentro do mapeamento.
        unsafe { &*self.ctrl.cast::<AtomicU64>() }
    }

    fn tail(&self) -> &AtomicU64 {
        // SAFETY: +64 do bloco de controle, alinhado a 8 bytes dentro do mapeamento.
        unsafe { &*self.ctrl.wrapping_add(64).cast::<AtomicU64>() }
    }

    fn data(&self) -> *mut u8 {
        self.ctrl.wrapping_add(CTRL)
    }

    fn pos(&self, index: u64) -> usize {
        (index % self.cap as u64) as usize
    }

    /// Publica o cursor do produtor (release): os registros escritos antes ficam visíveis ao consumidor.
    pub fn publish_tail(&self, tail: u64) {
        self.tail().store(tail, Release);
    }

    /// Bytes de pulo (marcador até o fim do buffer, 0 se não houver) e de registro para `len` bytes a partir de `tail`.
    fn footprint(&self, tail: u64, len: usize) -> (usize, usize) {
        let rec = rec_size(len);
        let to_end = self.cap - self.pos(tail);
        let skip = if rec > to_end { to_end } else { 0 };
        (skip, rec)
    }

    /// Cabe um registro de `len` bytes a partir de `tail` agora (com o `head` atual)? Não espera.
    pub fn fits(&self, tail: u64, len: usize) -> bool {
        let (skip, rec) = self.footprint(tail, len);
        let used = tail - self.head().load(Acquire);
        self.cap as u64 - used >= (skip + rec) as u64
    }

    /// Escreve um registro a partir do cursor local `tail` (do produtor). Não publica: chame `publish_tail`.
    /// Precondição: `fits` é verdadeiro; quem espera deve drenar o consumidor enquanto isso.
    pub fn push(&self, tail: &mut u64, payload: &[u8]) -> Res<()> {
        let len = payload.len();
        if rec_size(len) > self.cap {
            return Err(format!("mensagem de {len} bytes não cabe no anel").into());
        }
        if !self.fits(*tail, len) {
            return Err("anel sem espaço (produtor deve esperar com fits)".into());
        }
        let (skip, rec) = self.footprint(*tail, len);
        if skip > 0 {
            self.write_u32(self.pos(*tail), PAD_MARK);
            *tail += skip as u64;
        }
        self.write_record(self.pos(*tail), payload);
        *tail += rec as u64;
        Ok(())
    }

    fn write_record(&self, pos: usize, payload: &[u8]) {
        let len = payload.len();
        self.write_u32(pos, len as u32);
        // SAFETY: pos + rec_size(len) <= cap (o registro não atravessa o fim); a região é só do produtor até publish_tail.
        unsafe {
            let body = self.data().add(pos + 4);
            ptr::copy_nonoverlapping(payload.as_ptr(), body, len);
            ptr::write_bytes(body.add(len), 0, rec_size(len) - 4 - len);
        }
    }

    fn write_u32(&self, pos: usize, value: u32) {
        // SAFETY: pos é múltiplo de 8 e pos + 4 <= cap; região ainda não publicada, só o produtor escreve.
        unsafe { self.data().add(pos).cast::<[u8; 4]>().write(value.to_le_bytes()) }
    }

    fn read_u32(&self, pos: usize) -> u32 {
        // SAFETY: pos é múltiplo de 8 e pos + 4 <= cap; bytes já publicados pelo produtor (acquire no tail).
        u32::from_le_bytes(unsafe { self.data().add(pos).cast::<[u8; 4]>().read() })
    }

    /// Consome os registros publicados a partir do cursor local `head` (do consumidor), chamando
    /// `on_msg` por registro, e publica `head` ao fim. Devolve quantos registros foram consumidos.
    pub fn drain(&self, head: &mut u64, mut on_msg: impl FnMut(&[u8]) -> Res<()>) -> Res<usize> {
        let tail = self.tail().load(Acquire);
        let mut count = 0;
        while *head < tail {
            let pos = self.pos(*head);
            let len = self.read_u32(pos);
            if len == PAD_MARK {
                *head += (self.cap - pos) as u64;
                continue;
            }
            let rec = rec_size(len as usize);
            if rec > self.cap - pos || *head + rec as u64 > tail {
                return Err("registro inválido no anel (protocolo violado)".into());
            }
            // SAFETY: pos + 4 + len <= cap (garantido pela checagem de rec acima); bytes publicados e ainda não liberados pelo head.
            let payload = unsafe { std::slice::from_raw_parts(self.data().add(pos + 4), len as usize) };
            on_msg(payload)?;
            *head += rec as u64;
            count += 1;
        }
        self.head().store(*head, Release);
        Ok(count)
    }
}

/// Espera `ready` com spin por ~20 µs e depois yield. Erro se passar de STALL (filho morto ou travado).
pub fn wait_until(what: &str, mut ready: impl FnMut() -> Res<bool>) -> Res<()> {
    let start = Instant::now();
    while !ready()? {
        let waited = start.elapsed();
        if waited > STALL {
            return Err(format!("tempo esgotado esperando {what}").into());
        }
        if waited < SPIN_WINDOW {
            hint::spin_loop();
        } else {
            thread::yield_now();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{Checker, fill};

    const TEST_CAP: usize = 4096;
    const TEST_MSGS: usize = 5000;

    fn test_len(seq: u64) -> usize {
        (seq % 201) as usize
    }

    fn produce(ring: &Ring<'_>) -> Res<()> {
        let mut tail = 0;
        let mut buf = [0u8; 256];
        for seq in 0..TEST_MSGS as u64 {
            let len = test_len(seq);
            fill(&mut buf[..len], seq);
            wait_until("espaço no anel de teste", || Ok(ring.fits(tail, len)))?;
            ring.push(&mut tail, &buf[..len])?;
            ring.publish_tail(tail);
        }
        Ok(())
    }

    fn consume(ring: &Ring<'_>) -> Res<()> {
        let mut head = 0;
        let mut checker = Checker::new(test_len);
        wait_until("consumidor do teste", || {
            ring.drain(&mut head, |msg| checker.check(msg))?;
            Ok(checker.received() == TEST_MSGS as u64)
        })
    }

    #[test]
    fn ring_roundtrip_with_wrap_and_pad() -> Res<()> {
        let path = std::env::temp_dir().join(format!("p3-ring-test-{}.bin", std::process::id()));
        let shared = Shared::create(&path, TEST_CAP)?;
        let ring = shared.ring_a();
        let (sent, got) = thread::scope(|s| {
            let producer = s.spawn(|| produce(&ring));
            let consumed = consume(&ring);
            (producer.join().unwrap(), consumed)
        });
        drop(ring);
        drop(shared);
        std::fs::remove_file(&path)?;
        sent?;
        got
    }
}
