//! Embedding provider abstraction.
//!
//! v0.1 ships a deterministic hash-bucket embedder so the memory engine has
//! something to test against without dragging an LLM into the unit-test loop.
//! The real `nomic-embed-text` provider lands in `blues-model` (#9) and is
//! plugged in here later via `Box<dyn Embedder>`.

use async_trait::async_trait;
use blues_core::Result;

/// Fixed dimension for v0.1. Picked to be small enough that storage overhead
/// is negligible and large enough that hash collisions don't dominate.
pub const EMBED_DIM: usize = 128;

#[async_trait]
pub trait Embedder: Send + Sync {
    fn dim(&self) -> usize;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// Deterministic, content-addressed embedding for tests and offline mode.
///
/// Each token (whitespace-split, lowercased, trimmed) hashes into a single
/// bucket; the resulting vector is L2-normalised so cosine similarity falls
/// back to a dot product.
#[derive(Debug, Clone, Default)]
pub struct HashEmbedder;

impl HashEmbedder {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Embedder for HashEmbedder {
    fn dim(&self) -> usize { EMBED_DIM }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut v = vec![0f32; EMBED_DIM];
        for tok in tokenize(text) {
            let h = fnv1a(tok.as_bytes()) as usize;
            v[h % EMBED_DIM] += 1.0;
        }
        l2_normalise(&mut v);
        Ok(v)
    }
}

fn tokenize(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn l2_normalise(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in v {
            *x /= n;
        }
    }
}

/// Cosine similarity for two embeddings of equal length. Inputs are assumed
/// normalised (as produced by `HashEmbedder`); for safety we still divide.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let d = (na.sqrt() * nb.sqrt()).max(f32::EPSILON);
    dot / d
}

/// Pack an `f32` slice into little-endian bytes for sqlite BLOB storage.
pub fn pack(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

/// Inverse of [`pack`]. Returns an error if the byte length isn't a multiple
/// of 4 — caller turns that into `BluesError::Internal`.
pub fn unpack(bytes: &[u8]) -> std::result::Result<Vec<f32>, &'static str> {
    if !bytes.len().is_multiple_of(4) {
        return Err("embedding blob length not a multiple of 4");
    }
    let mut out = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_embedder_is_deterministic() {
        let e = HashEmbedder;
        let a = e.embed("hello world").await.unwrap();
        let b = e.embed("hello world").await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn hash_embedder_separates_distinct_text() {
        let e = HashEmbedder;
        let a = e.embed("logout flow").await.unwrap();
        let b = e.embed("totally unrelated payment retry").await.unwrap();
        assert!(cosine(&a, &b) < 0.5, "cos {} too high", cosine(&a, &b));
    }

    #[tokio::test]
    async fn hash_embedder_overlap_is_higher_than_disjoint() {
        let e = HashEmbedder;
        let q = e.embed("logout flow").await.unwrap();
        let near = e.embed("logout cleanup flow").await.unwrap();
        let far = e.embed("payment retry tax").await.unwrap();
        assert!(cosine(&q, &near) > cosine(&q, &far));
    }

    #[test]
    fn pack_unpack_roundtrip() {
        let v = vec![0.0, 1.0, -2.5, 0.125];
        let bytes = pack(&v);
        let back = unpack(&bytes).unwrap();
        assert_eq!(v, back);
    }
}
