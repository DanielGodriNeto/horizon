//! Mensagens do bench: 8 bytes do número (u64 LE) seguidos de padrão fixo; verificação sem alocação.
use crate::Res;

const PAD: u8 = 0xA5;

fn byte_at(seq: u64, idx: usize) -> u8 {
    if idx < 8 { seq.to_le_bytes()[idx] } else { PAD }
}

/// Preenche `buf` com a mensagem número `seq`.
pub fn fill(buf: &mut [u8], seq: u64) {
    for (idx, byte) in buf.iter_mut().enumerate() {
        *byte = byte_at(seq, idx);
    }
}

/// Confere, na ordem de chegada, que cada mensagem é a esperada (número, tamanho e conteúdo).
pub struct Checker {
    next: u64,
    len_of: fn(u64) -> usize,
}

impl Checker {
    pub fn new(len_of: fn(u64) -> usize) -> Checker {
        Checker { next: 0, len_of }
    }

    pub fn received(&self) -> u64 {
        self.next
    }

    pub fn check(&mut self, msg: &[u8]) -> Res<()> {
        let want = (self.len_of)(self.next);
        if msg.len() != want {
            return Err(format!("mensagem {}: tamanho {} (esperado {want})", self.next, msg.len()).into());
        }
        let bad = msg.iter().enumerate().find(|&(idx, &byte)| byte != byte_at(self.next, idx));
        if let Some((idx, _)) = bad {
            return Err(format!("mensagem {}: byte {idx} divergente", self.next).into());
        }
        self.next += 1;
        Ok(())
    }
}
