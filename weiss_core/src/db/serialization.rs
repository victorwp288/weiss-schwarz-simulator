use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::store::CardDb;

const WSDB_MAGIC: &[u8; 4] = b"WSDB";
/// Current wsdb schema version.
pub const WSDB_SCHEMA_VERSION: u32 = 1;

impl CardDb {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = fs::read(&path)
            .with_context(|| format!("Failed to read card db {:?}", path.as_ref()))?;
        Self::from_wsdb_bytes(&bytes)
    }

    pub fn from_wsdb_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            anyhow::bail!("Card db file too small");
        }
        if &bytes[0..4] != WSDB_MAGIC {
            anyhow::bail!("Card db magic mismatch; expected WSDB header");
        }
        let version = u32::from_le_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Card db header missing version bytes"))?,
        );
        if version != WSDB_SCHEMA_VERSION {
            anyhow::bail!(
                "Unsupported card db schema version {version}, expected {WSDB_SCHEMA_VERSION}"
            );
        }
        let payload = &bytes[8..];
        Self::from_postcard_payload(payload)
    }

    pub fn from_postcard_payload(payload: &[u8]) -> Result<Self> {
        let mut db: CardDb =
            postcard::from_bytes(payload).context("Failed to decode card db payload")?;
        db.build_index()?;
        Ok(db)
    }

    pub fn schema_version() -> u32 {
        WSDB_SCHEMA_VERSION
    }

    pub fn to_bytes_with_header(&self) -> Result<Vec<u8>> {
        let payload = postcard::to_stdvec(self)?;
        let mut out = Vec::with_capacity(8 + payload.len());
        out.extend_from_slice(WSDB_MAGIC);
        out.extend_from_slice(&WSDB_SCHEMA_VERSION.to_le_bytes());
        out.extend_from_slice(&payload);
        Ok(out)
    }
}
