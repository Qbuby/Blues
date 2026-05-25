//! SQLite storage layer.
//!
//! All three cognitive layers (episodic / semantic / procedural) plus the
//! inbox candidate area live in one sqlite file per project. FTS5 mirrors
//! provide full-text search; embeddings ride alongside as BLOB columns
//! (sqlite-vec is on the v0.2 menu — for v0.1 we cosine-rank in-process).
//!
//! See ARCHITECTURE.md §2.4 + §4.

use blues_core::{
    BluesError, EpisodeId, FactId, InboxItemId, MemoryType, NodeId, PlanId, ProjectId,
    Provenance, Result, Source, Step,
};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use time::OffsetDateTime;

use crate::embed::{pack, unpack, EMBED_DIM};

/// One sqlite connection guarded by a mutex. Memory traffic is low-volume
/// (human-paced approvals + agent extractions) so a single connection in
/// WAL mode is fine; we'll move to a pool if/when contention bites.
#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

/// Internal row representation — owned strings, raw timestamps, untyped ids.
/// We translate to typed `MemoryHit` etc. at the trait surface.
pub struct ConfirmedRow {
    pub id: String,
    pub kind: MemoryType,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub provenance: Provenance,
    pub confidence: f32,
}

pub struct InboxRow {
    pub id: InboxItemId,
    pub kind: MemoryType,
    pub content: String,
    pub confidence: f32,
    pub provenance: Provenance,
    pub plan_id: Option<PlanId>,
    pub node_id: Option<NodeId>,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(map_sql)?;
        Self::init(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(map_sql)?;
        Self::init(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    fn init(conn: &Connection) -> Result<()> {
        conn.execute_batch(SCHEMA).map_err(map_sql)?;
        Ok(())
    }

    // ── Inbox ────────────────────────────────────────────────────────────

    pub fn inbox_insert(
        &self,
        project: ProjectId,
        item: &InboxRow,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let prov = serde_json::to_string(&item.provenance)
            .map_err(|e| BluesError::Internal(format!("provenance encode: {e}")))?;
        conn.execute(
            "INSERT INTO inbox(id, project_id, kind, content, confidence, provenance, plan_id, node_id, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                item.id.to_string(),
                project.to_string(),
                kind_to_str(item.kind),
                item.content,
                item.confidence,
                prov,
                item.plan_id.map(|p| p.to_string()),
                item.node_id.map(|n| n.to_string()),
                now_rfc3339()?,
            ],
        ).map_err(map_sql)?;
        Ok(())
    }

    pub fn inbox_list(
        &self,
        project: ProjectId,
        kind: Option<MemoryType>,
        min_confidence: Option<f32>,
        limit: usize,
    ) -> Result<Vec<InboxRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, kind, content, confidence, provenance, plan_id, node_id
             FROM inbox
             WHERE project_id = ?1
               AND (?2 IS NULL OR kind = ?2)
               AND (?3 IS NULL OR confidence >= ?3)
             ORDER BY created_at ASC
             LIMIT ?4"
        ).map_err(map_sql)?;
        let rows = stmt.query_map(
            params![
                project.to_string(),
                kind.map(kind_to_str),
                min_confidence,
                limit as i64,
            ],
            |row| {
                let id: String = row.get(0)?;
                let kind: String = row.get(1)?;
                let content: String = row.get(2)?;
                let confidence: f32 = row.get(3)?;
                let prov: String = row.get(4)?;
                let plan_id: Option<String> = row.get(5)?;
                let node_id: Option<String> = row.get(6)?;
                Ok((id, kind, content, confidence, prov, plan_id, node_id))
            },
        ).map_err(map_sql)?;

        let mut out = Vec::new();
        for row in rows {
            let (id, kind, content, confidence, prov, plan_id, node_id) =
                row.map_err(map_sql)?;
            out.push(InboxRow {
                id: parse_inbox_id(&id)?,
                kind: kind_from_str(&kind)?,
                content,
                confidence,
                provenance: serde_json::from_str(&prov)
                    .map_err(|e| BluesError::Internal(format!("provenance decode: {e}")))?,
                plan_id: opt_parse(plan_id, "plan_id")?,
                node_id: opt_parse(node_id, "node_id")?,
            });
        }
        Ok(out)
    }

    pub fn inbox_get(&self, item: InboxItemId) -> Result<Option<InboxFull>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT project_id, kind, content, confidence, provenance, plan_id, node_id
             FROM inbox WHERE id = ?1",
            params![item.to_string()],
            |row| {
                let project_id: String = row.get(0)?;
                let kind: String = row.get(1)?;
                let content: String = row.get(2)?;
                let confidence: f32 = row.get(3)?;
                let prov: String = row.get(4)?;
                let plan_id: Option<String> = row.get(5)?;
                let node_id: Option<String> = row.get(6)?;
                Ok((project_id, kind, content, confidence, prov, plan_id, node_id))
            },
        )
        .optional()
        .map_err(map_sql)?
        .map(|(project_id, kind, content, confidence, prov, plan_id, node_id)| {
            Ok(InboxFull {
                project: ProjectId::from_str(&project_id)
                    .map_err(|e| BluesError::Internal(format!("project_id decode: {e}")))?,
                row: InboxRow {
                    id: item,
                    kind: kind_from_str(&kind)?,
                    content,
                    confidence,
                    provenance: serde_json::from_str(&prov)
                        .map_err(|e| BluesError::Internal(format!("provenance decode: {e}")))?,
                    plan_id: opt_parse(plan_id, "plan_id")?,
                    node_id: opt_parse(node_id, "node_id")?,
                },
            })
        })
        .transpose()
    }

    pub fn inbox_delete(&self, item: InboxItemId) -> Result<()> {
        let conn = self.conn.lock();
        let n = conn.execute("DELETE FROM inbox WHERE id = ?1", params![item.to_string()])
            .map_err(map_sql)?;
        if n == 0 {
            return Err(BluesError::NotFound(format!("inbox item {item}")));
        }
        Ok(())
    }

    // ── Confirmed: facts (semantic + procedural) and episodes ────────────

    #[allow(clippy::too_many_arguments)]
    pub fn fact_insert(
        &self,
        project: ProjectId,
        id: FactId,
        kind: MemoryType,
        content: &str,
        embedding: &[f32],
        provenance: &Provenance,
        confidence: f32,
    ) -> Result<()> {
        debug_assert!(matches!(kind, MemoryType::Semantic | MemoryType::Procedural));
        let conn = self.conn.lock();
        let prov = serde_json::to_string(provenance)
            .map_err(|e| BluesError::Internal(format!("provenance encode: {e}")))?;
        conn.execute(
            "INSERT INTO facts(id, project_id, kind, content, embedding, provenance, confidence, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id.to_string(),
                project.to_string(),
                kind_to_str(kind),
                content,
                pack(embedding),
                prov,
                confidence,
                now_rfc3339()?,
            ],
        ).map_err(map_sql)?;
        conn.execute(
            "INSERT INTO facts_fts(rowid, content) VALUES((SELECT rowid FROM facts WHERE id = ?1), ?2)",
            params![id.to_string(), content],
        ).map_err(map_sql)?;
        Ok(())
    }

    pub fn episode_insert(
        &self,
        project: ProjectId,
        id: EpisodeId,
        content: &str,
        embedding: &[f32],
        provenance: &Provenance,
        confidence: f32,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let prov = serde_json::to_string(provenance)
            .map_err(|e| BluesError::Internal(format!("provenance encode: {e}")))?;
        conn.execute(
            "INSERT INTO episodes(id, project_id, content, embedding, provenance, confidence, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id.to_string(),
                project.to_string(),
                content,
                pack(embedding),
                prov,
                confidence,
                now_rfc3339()?,
            ],
        ).map_err(map_sql)?;
        conn.execute(
            "INSERT INTO episodes_fts(rowid, content) VALUES((SELECT rowid FROM episodes WHERE id = ?1), ?2)",
            params![id.to_string(), content],
        ).map_err(map_sql)?;
        Ok(())
    }

    /// Pull every confirmed row for a project, optionally filtered by kind.
    /// Used by the multi-route recall — vector cosine ranking is done in
    /// memory (sqlite-vec migration is v0.2).
    pub fn confirmed_all(
        &self,
        project: ProjectId,
        kind: Option<MemoryType>,
    ) -> Result<Vec<ConfirmedRow>> {
        let conn = self.conn.lock();
        let mut out = Vec::new();
        if matches!(kind, None | Some(MemoryType::Semantic) | Some(MemoryType::Procedural)) {
            let mut stmt = conn.prepare(
                "SELECT id, kind, content, embedding, provenance, confidence
                 FROM facts
                 WHERE project_id = ?1 AND (?2 IS NULL OR kind = ?2)"
            ).map_err(map_sql)?;
            let kind_str = match kind {
                Some(MemoryType::Semantic) => Some(kind_to_str(MemoryType::Semantic)),
                Some(MemoryType::Procedural) => Some(kind_to_str(MemoryType::Procedural)),
                _ => None,
            };
            let rows = stmt.query_map(params![project.to_string(), kind_str], read_confirmed_row)
                .map_err(map_sql)?;
            for r in rows {
                out.push(decode_confirmed(r.map_err(map_sql)?)?);
            }
        }
        if matches!(kind, None | Some(MemoryType::Episodic)) {
            let mut stmt = conn.prepare(
                "SELECT id, content, embedding, provenance, confidence
                 FROM episodes
                 WHERE project_id = ?1"
            ).map_err(map_sql)?;
            let rows = stmt.query_map(params![project.to_string()], |row| {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let embedding: Option<Vec<u8>> = row.get(2)?;
                let prov: String = row.get(3)?;
                let confidence: f32 = row.get(4)?;
                Ok((id, content, embedding, prov, confidence))
            }).map_err(map_sql)?;
            for r in rows {
                let (id, content, emb, prov, conf) = r.map_err(map_sql)?;
                out.push(ConfirmedRow {
                    id,
                    kind: MemoryType::Episodic,
                    content,
                    embedding: emb.map(|b| unpack(&b)).transpose()
                        .map_err(|e| BluesError::Internal(format!("embedding decode: {e}")))?,
                    provenance: serde_json::from_str(&prov)
                        .map_err(|e| BluesError::Internal(format!("provenance decode: {e}")))?,
                    confidence: conf,
                });
            }
        }
        Ok(out)
    }

    /// FTS5 BM25 candidate ids. Returns `(id, kind, bm25_score)` ordered by
    /// best match first. We deliberately leak ids out so the caller can fuse
    /// with vector results without re-fetching content twice.
    pub fn fts_search(
        &self,
        project: ProjectId,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(String, MemoryType, f32)>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let q = sanitize_fts(query);
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock();

        let mut out = Vec::new();

        let mut s = conn.prepare(
            "SELECT facts.id, facts.kind, bm25(facts_fts)
             FROM facts_fts JOIN facts ON facts_fts.rowid = facts.rowid
             WHERE facts.project_id = ?1 AND facts_fts MATCH ?2
             ORDER BY bm25(facts_fts) ASC
             LIMIT ?3"
        ).map_err(map_sql)?;
        let rows = s.query_map(params![project.to_string(), &q, limit as i64], |row| {
            let id: String = row.get(0)?;
            let kind: String = row.get(1)?;
            let score: f64 = row.get(2)?;
            Ok((id, kind, score as f32))
        }).map_err(map_sql)?;
        for r in rows {
            let (id, k, sc) = r.map_err(map_sql)?;
            out.push((id, kind_from_str(&k)?, sc));
        }

        let mut s = conn.prepare(
            "SELECT episodes.id, bm25(episodes_fts)
             FROM episodes_fts JOIN episodes ON episodes_fts.rowid = episodes.rowid
             WHERE episodes.project_id = ?1 AND episodes_fts MATCH ?2
             ORDER BY bm25(episodes_fts) ASC
             LIMIT ?3"
        ).map_err(map_sql)?;
        let rows = s.query_map(params![project.to_string(), &q, limit as i64], |row| {
            let id: String = row.get(0)?;
            let score: f64 = row.get(1)?;
            Ok((id, score as f32))
        }).map_err(map_sql)?;
        for r in rows {
            let (id, sc) = r.map_err(map_sql)?;
            out.push((id, MemoryType::Episodic, sc));
        }
        Ok(out)
    }

    pub fn fact_get(&self, id: FactId) -> Result<ConfirmedRow> {
        let conn = self.conn.lock();
        let r = conn.query_row(
            "SELECT id, kind, content, embedding, provenance, confidence
             FROM facts WHERE id = ?1",
            params![id.to_string()],
            read_confirmed_row,
        ).optional().map_err(map_sql)?;
        match r {
            Some(t) => decode_confirmed(t),
            None => Err(BluesError::NotFound(format!("fact {id}"))),
        }
    }

    pub fn episode_get(&self, id: EpisodeId) -> Result<ConfirmedRow> {
        let conn = self.conn.lock();
        let r = conn.query_row(
            "SELECT id, content, embedding, provenance, confidence
             FROM episodes WHERE id = ?1",
            params![id.to_string()],
            |row| {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let embedding: Option<Vec<u8>> = row.get(2)?;
                let prov: String = row.get(3)?;
                let confidence: f32 = row.get(4)?;
                Ok((id, content, embedding, prov, confidence))
            },
        ).optional().map_err(map_sql)?;
        match r {
            Some((id, content, emb, prov, conf)) => Ok(ConfirmedRow {
                id,
                kind: MemoryType::Episodic,
                content,
                embedding: emb.map(|b| unpack(&b)).transpose()
                    .map_err(|e| BluesError::Internal(format!("embedding decode: {e}")))?,
                provenance: serde_json::from_str(&prov)
                    .map_err(|e| BluesError::Internal(format!("provenance decode: {e}")))?,
                confidence: conf,
            }),
            None => Err(BluesError::NotFound(format!("episode {id}"))),
        }
    }

    pub fn fact_provenance(&self, id: FactId) -> Result<Provenance> {
        Ok(self.fact_get(id)?.provenance)
    }

    /// Decay confidence of every confirmed row toward zero by `factor` (0..1).
    /// Returns the number of rows touched.
    pub fn decay(&self, project: ProjectId, factor: f32) -> Result<usize> {
        let conn = self.conn.lock();
        let f = factor.clamp(0.0, 1.0);
        let n1 = conn.execute(
            "UPDATE facts SET confidence = confidence * ?1 WHERE project_id = ?2",
            params![f, project.to_string()],
        ).map_err(map_sql)?;
        let n2 = conn.execute(
            "UPDATE episodes SET confidence = confidence * ?1 WHERE project_id = ?2",
            params![f, project.to_string()],
        ).map_err(map_sql)?;
        Ok(n1 + n2)
    }
}

pub struct InboxFull {
    pub project: ProjectId,
    pub row: InboxRow,
}

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS inbox (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL,
    kind        TEXT NOT NULL,
    content     TEXT NOT NULL,
    confidence  REAL NOT NULL,
    provenance  TEXT NOT NULL,
    plan_id     TEXT,
    node_id     TEXT,
    created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_inbox_project ON inbox(project_id, created_at);

CREATE TABLE IF NOT EXISTS facts (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL,
    kind        TEXT NOT NULL,             -- semantic | procedural
    content     TEXT NOT NULL,
    embedding   BLOB,
    provenance  TEXT NOT NULL,
    confidence  REAL NOT NULL,
    created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_facts_project ON facts(project_id, kind);

CREATE TABLE IF NOT EXISTS episodes (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL,
    content     TEXT NOT NULL,
    embedding   BLOB,
    provenance  TEXT NOT NULL,
    confidence  REAL NOT NULL,
    created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_episodes_project ON episodes(project_id, created_at);

CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
    content, content='facts', content_rowid='rowid', tokenize='porter unicode61'
);
CREATE VIRTUAL TABLE IF NOT EXISTS episodes_fts USING fts5(
    content, content='episodes', content_rowid='rowid', tokenize='porter unicode61'
);
"#;

// ── helpers ─────────────────────────────────────────────────────────────

fn map_sql(e: rusqlite::Error) -> BluesError {
    BluesError::Internal(format!("sqlite: {e}"))
}

fn now_rfc3339() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| BluesError::Internal(format!("time fmt: {e}")))
}

fn kind_to_str(k: MemoryType) -> &'static str {
    match k {
        MemoryType::Episodic => "episodic",
        MemoryType::Semantic => "semantic",
        MemoryType::Procedural => "procedural",
    }
}

fn kind_from_str(s: &str) -> Result<MemoryType> {
    match s {
        "episodic" => Ok(MemoryType::Episodic),
        "semantic" => Ok(MemoryType::Semantic),
        "procedural" => Ok(MemoryType::Procedural),
        other => Err(BluesError::Internal(format!("bad memory kind in db: {other}"))),
    }
}

fn parse_inbox_id(s: &str) -> Result<InboxItemId> {
    InboxItemId::from_str(s).map_err(|e| BluesError::Internal(format!("inbox id: {e}")))
}

fn opt_parse<T>(s: Option<String>, label: &str) -> Result<Option<T>>
where T: FromStr,
      <T as FromStr>::Err: std::fmt::Display,
{
    match s {
        None => Ok(None),
        Some(x) => x.parse().map(Some)
            .map_err(|e: <T as FromStr>::Err| {
                BluesError::Internal(format!("{label} decode: {e}"))
            }),
    }
}

type ConfirmedRowTuple = (String, String, String, Option<Vec<u8>>, String, f32);

fn read_confirmed_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConfirmedRowTuple> {
    let id: String = row.get(0)?;
    let kind: String = row.get(1)?;
    let content: String = row.get(2)?;
    let embedding: Option<Vec<u8>> = row.get(3)?;
    let prov: String = row.get(4)?;
    let confidence: f32 = row.get(5)?;
    Ok((id, kind, content, embedding, prov, confidence))
}

fn decode_confirmed(
    (id, kind, content, embedding, prov, confidence): ConfirmedRowTuple,
) -> Result<ConfirmedRow> {
    Ok(ConfirmedRow {
        id,
        kind: kind_from_str(&kind)?,
        content,
        embedding: embedding.map(|b| unpack(&b)).transpose()
            .map_err(|e| BluesError::Internal(format!("embedding decode: {e}")))?,
        provenance: serde_json::from_str(&prov)
            .map_err(|e| BluesError::Internal(format!("provenance decode: {e}")))?,
        confidence,
    })
}

/// Defang FTS5 metacharacters. v0.1 ships a deliberately dumb sanitiser:
/// strip anything that isn't alphanumeric, then OR-join the resulting tokens.
/// A real query parser is on the v0.2 menu.
fn sanitize_fts(q: &str) -> String {
    let toks: Vec<String> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect();
    toks.join(" OR ")
}

/// Append an ingestion step to the provenance chain. Used by the engine on
/// every save/approve to record `daemon`/`memory` causality.
pub fn attach_step(p: &mut Provenance, action: &str, by: &str) {
    p.chain.push(Step {
        at: OffsetDateTime::now_utc(),
        action: action.into(),
        by: by.into(),
    });
}

/// Convenience: build a default provenance for a `MemoryWrite` with no
/// caller-supplied chain. Records the source kind we can infer.
pub fn default_provenance(source: Option<&str>) -> Provenance {
    let sources = match source {
        Some(s) => vec![Source::User { input: s.to_string() }],
        None => vec![],
    };
    Provenance { sources, chain: Vec::new() }
}

/// Sanity check that an embedding has the expected dimension.
pub fn ensure_dim(v: &[f32]) -> Result<()> {
    if v.len() != EMBED_DIM {
        return Err(BluesError::Internal(format!(
            "embedding dim mismatch: got {}, want {EMBED_DIM}", v.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::{Embedder, HashEmbedder};

    fn store() -> SqliteStore { SqliteStore::open_in_memory().unwrap() }

    #[tokio::test]
    async fn schema_inits_clean_in_memory() {
        let _ = store();
    }

    #[tokio::test]
    async fn inbox_insert_then_list_then_get_then_delete() {
        let s = store();
        let p = ProjectId::new();
        let item = InboxRow {
            id: InboxItemId::new(),
            kind: MemoryType::Semantic,
            content: "auth.ts owns logout flow".into(),
            confidence: 0.7,
            provenance: default_provenance(Some("agent:smart")),
            plan_id: None,
            node_id: None,
        };
        s.inbox_insert(p, &item).unwrap();

        let listed = s.inbox_list(p, None, None, 10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].content, "auth.ts owns logout flow");

        let got = s.inbox_get(item.id).unwrap().expect("found");
        assert_eq!(got.project, p);
        assert_eq!(got.row.kind, MemoryType::Semantic);

        s.inbox_delete(item.id).unwrap();
        assert!(s.inbox_get(item.id).unwrap().is_none());
    }

    #[tokio::test]
    async fn fact_insert_get_roundtrip() {
        let s = store();
        let e = HashEmbedder;
        let p = ProjectId::new();
        let id = FactId::new();
        let emb = e.embed("auth").await.unwrap();
        s.fact_insert(p, id, MemoryType::Semantic, "auth.ts owns logout", &emb,
                      &default_provenance(None), 0.8).unwrap();
        let got = s.fact_get(id).unwrap();
        assert_eq!(got.content, "auth.ts owns logout");
        assert_eq!(got.kind, MemoryType::Semantic);
        assert!(got.embedding.is_some());
    }

    #[tokio::test]
    async fn fts_finds_only_project_match() {
        let s = store();
        let e = HashEmbedder;
        let p1 = ProjectId::new();
        let p2 = ProjectId::new();
        let id = FactId::new();
        s.fact_insert(p1, id, MemoryType::Semantic, "logout flow lives in auth.ts",
                      &e.embed("auth").await.unwrap(),
                      &default_provenance(None), 0.9).unwrap();
        let id2 = FactId::new();
        s.fact_insert(p2, id2, MemoryType::Semantic, "billing logic in pay.ts",
                      &e.embed("pay").await.unwrap(),
                      &default_provenance(None), 0.9).unwrap();

        let hits = s.fts_search(p1, "logout", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, id.to_string());

        let hits2 = s.fts_search(p2, "logout", 10).unwrap();
        assert!(hits2.is_empty());
    }

    #[tokio::test]
    async fn decay_reduces_confidence() {
        let s = store();
        let e = HashEmbedder;
        let p = ProjectId::new();
        let id = FactId::new();
        s.fact_insert(p, id, MemoryType::Semantic, "x", &e.embed("x").await.unwrap(),
                      &default_provenance(None), 1.0).unwrap();
        let touched = s.decay(p, 0.5).unwrap();
        assert!(touched >= 1);
        let got = s.fact_get(id).unwrap();
        assert!((got.confidence - 0.5).abs() < 1e-5);
    }
}
