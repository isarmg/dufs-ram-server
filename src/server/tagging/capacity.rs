//! New facts are admitted under the same SQLite writer transaction as their insertion.
use anyhow::{Result, ensure};
use sqlx::SqliteConnection;
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub files: i64,
    pub tags: i64,
    pub relations: i64,
    pub database_bytes: u64,
    pub free_floor: u64,
}
impl Limits {
    pub const PRODUCTION: Self = Self {
        files: 1_000_000,
        tags: 10_000,
        relations: 1_000_000,
        database_bytes: 8 * 1024 * 1024 * 1024,
        free_floor: 1024 * 1024 * 1024,
    };
}
#[derive(Debug, thiserror::Error)]
#[error("persistent tag capacity is exhausted")]
pub(super) struct Exhausted;

pub(super) async fn storage(
    connection: &mut SqliteConnection,
    path: &Path,
    new_bytes: u64,
    limits: Limits,
) -> Result<()> {
    let pages: i64 = sqlx::query_scalar("PRAGMA page_count")
        .fetch_one(&mut *connection)
        .await?;
    let page_size: i64 = sqlx::query_scalar("PRAGMA page_size")
        .fetch_one(&mut *connection)
        .await?;
    let mut current = u64::try_from(pages)?
        .checked_mul(u64::try_from(page_size)?)
        .ok_or(Exhausted)?;
    for suffix in ["-wal", "-journal"] {
        let mut journal = path.as_os_str().to_os_string();
        journal.push(suffix);
        match std::fs::symlink_metadata(journal) {
            Ok(metadata) => {
                ensure!(
                    metadata.is_file() && !metadata.file_type().is_symlink(),
                    "invalid tag journal identity"
                );
                current = current.checked_add(metadata.len()).ok_or(Exhausted)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("tag database has no parent"))?;
    let stats = rustix::fs::statvfs(parent)?;
    let available = stats.f_bavail.saturating_mul(stats.f_frsize);
    // Retain room for removals, confirmations, and recovery metadata, rather than accepting
    // new facts whose later bookkeeping would have nowhere to become durable.
    let reserve = 64 * 1024 * 1024;
    if current.saturating_add(new_bytes).saturating_add(reserve) > limits.database_bytes
        || available
            < limits
                .free_floor
                .saturating_add(new_bytes)
                .saturating_add(reserve)
    {
        return Err(Exhausted.into());
    }
    Ok(())
}
