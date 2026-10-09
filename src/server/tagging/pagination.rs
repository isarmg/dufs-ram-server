use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(super) const PAGE_SIZE: usize = 50;
pub(super) const MAX_CURSOR_BYTES: usize = 4096;
#[derive(Debug, thiserror::Error)]
#[error("invalid tag page cursor")]
pub(super) struct InvalidCursor;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cursor {
    version: u8,
    scope: String,
    pub reverse: bool,
    pub name: String,
    pub id: i64,
}
impl Cursor {
    pub fn decode(value: Option<&str>, scope: &str) -> Result<Option<Self>> {
        let Some(value) = value else {
            return Ok(None);
        };
        if value.is_empty() || value.len() > MAX_CURSOR_BYTES {
            return Err(InvalidCursor.into());
        }
        let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| InvalidCursor)?;
        let cursor: Self = serde_json::from_slice(&bytes).map_err(|_| InvalidCursor)?;
        if cursor.version != 1
            || cursor.scope != scope
            || cursor.id < 0
            || cursor.name.len() > 768
            || cursor.encode()? != value
        {
            return Err(InvalidCursor.into());
        }
        Ok(Some(cursor))
    }
    fn encode(&self) -> Result<String> {
        let value = URL_SAFE_NO_PAD.encode(serde_json::to_vec(self)?);
        if value.len() > MAX_CURSOR_BYTES {
            return Err(InvalidCursor.into());
        }
        Ok(value)
    }
    pub fn at(scope: &str, name: &str, id: i64, reverse: bool) -> Result<String> {
        Self {
            version: 1,
            scope: scope.to_owned(),
            reverse,
            name: name.to_owned(),
            id,
        }
        .encode()
    }
}
pub(super) fn scope(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    hex::encode(hash.finalize())
}
