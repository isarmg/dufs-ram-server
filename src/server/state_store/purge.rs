use super::model::validate_stored_path;
use super::*;

impl StoreWorker {
    pub(super) fn prepare_purge_job(&mut self, proposed: &StoredPurgeJob) -> Result<StorePurgeJob> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let mut transaction = self.connection.begin_with("BEGIN IMMEDIATE").await?;
            if let Some(existing) = load_purge_job(&mut transaction, proposed.key).await? {
                transaction.commit().await?;
                return Ok(
                    if existing.target_path == proposed.target_path
                        && existing.trash_path == proposed.trash_path
                        && existing.source_identity == proposed.source_identity
                        && existing.is_directory == proposed.is_directory
                    {
                        StorePurgeJob::Existing
                    } else {
                        StorePurgeJob::Conflict
                    },
                );
            }
            let conflicting_path: bool =
                sqlx::query("SELECT EXISTS(SELECT 1 FROM purge_jobs WHERE trash_path = ?1)")
                    .bind(proposed.trash_path.as_os_str().as_bytes())
                    .try_map(|row: sqlx::sqlite::SqliteRow| row.try_get(0))
                    .fetch_one(&mut *transaction)
                    .await?;
            if conflicting_path {
                transaction.commit().await?;
                return Ok(StorePurgeJob::Conflict);
            }
            let global_count: i64 = sqlx::query("SELECT COUNT(*) FROM purge_jobs")
                .try_map(|row: sqlx::sqlite::SqliteRow| row.try_get(0))
                .fetch_one(&mut *transaction)
                .await?;
            let owner_count: i64 =
                sqlx::query("SELECT COUNT(*) FROM purge_jobs WHERE owner_digest = ?1")
                    .bind(proposed.key.owner.as_slice())
                    .try_map(|row: sqlx::sqlite::SqliteRow| row.try_get(0))
                    .fetch_one(&mut *transaction)
                    .await?;
            if global_count >= self.limits.purge_capacity
                || owner_count >= self.limits.purge_per_owner
            {
                transaction.commit().await?;
                return Ok(StorePurgeJob::Full);
            }
            sqlx::query(
                "INSERT INTO purge_jobs(
                 owner_digest, job_id, target_path, trash_path,
                 source_device_be, source_inode_be, is_directory, state, attempts,
                 next_attempt_at_ms, created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?9, ?9)",
            )
            .bind(proposed.key.owner.as_slice())
            .bind(proposed.key.id.as_slice())
            .bind(proposed.target_path.as_os_str().as_bytes())
            .bind(proposed.trash_path.as_os_str().as_bytes())
            .bind(proposed.source_identity.device.to_be_bytes().as_slice())
            .bind(proposed.source_identity.inode.to_be_bytes().as_slice())
            .bind(i64::from(proposed.is_directory))
            .bind(PURGE_PREPARED)
            .bind(now)
            .execute(&mut *transaction)
            .await
            .map(|result| result.rows_affected())?;
            transaction.commit().await?;
            Ok(StorePurgeJob::Inserted)
        })
    }

    pub(super) fn prepared_purge_jobs(&mut self, limit: i64) -> Result<Vec<StoredPurgeJob>> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            query_purge_jobs(
                &mut self.connection,
                "SELECT owner_digest, job_id, target_path, trash_path,
                    source_device_be, source_inode_be, trash_revision,
                    is_directory, state, attempts
               FROM purge_jobs
              WHERE state = ?1
              ORDER BY created_at_ms, owner_digest, job_id
              LIMIT ?2",
                sql_arguments!(PURGE_PREPARED, limit),
            )
            .await
        })
    }

    pub(super) fn purge_jobs(&mut self, limit: i64) -> Result<Vec<StoredPurgeJob>> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            query_purge_jobs(
                &mut self.connection,
                "SELECT owner_digest, job_id, target_path, trash_path,
                    source_device_be, source_inode_be, trash_revision,
                    is_directory, state, attempts
               FROM purge_jobs
              ORDER BY created_at_ms, owner_digest, job_id
              LIMIT ?1",
                sql_arguments!(limit),
            )
            .await
        })
    }

    pub(super) fn state_path_is_bound(&mut self, path: &Path) -> Result<bool> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            sqlx::query(
                "SELECT EXISTS(
                     SELECT 1 FROM upload_sessions
                      WHERE stage_path = ?1
                        AND state IN (?2, ?3, ?4)
                     UNION ALL
                     SELECT 1 FROM purge_jobs WHERE trash_path = ?1
                 )",
            )
            .bind(path.as_os_str().as_bytes())
            .bind(UPLOAD_RUNNING)
            .bind(UPLOAD_COMMIT_STARTED)
            .bind(UPLOAD_AWAITING_CONFIRMATION)
            .try_map(|row: sqlx::sqlite::SqliteRow| row.try_get(0))
            .fetch_one(&mut self.connection)
            .await
            .map_err(Into::into)
        })
    }

    pub(super) fn state_blocking_paths(
        &mut self,
        after: Option<StatePathCursor>,
        limit: i64,
    ) -> Result<StatePathPage> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            query_state_blocking_paths(&mut self.connection, after, limit).await
        })
    }

    pub(super) fn mark_purge_job_ready(
        &mut self,
        key: PurgeJobKey,
        trash_revision: [u8; 32],
    ) -> Result<bool> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let changed = sqlx::query(
                "UPDATE purge_jobs
                SET state = ?1, trash_revision = ?2,
                    next_attempt_at_ms = ?3, updated_at_ms = ?3
              WHERE owner_digest = ?4 AND job_id = ?5 AND state = ?6",
            )
            .bind(PURGE_READY)
            .bind(trash_revision.as_slice())
            .bind(now)
            .bind(key.owner.as_slice())
            .bind(key.id.as_slice())
            .bind(PURGE_PREPARED)
            .execute(&mut self.connection)
            .await
            .map(|result| result.rows_affected())?;
            if changed == 1 {
                return Ok(true);
            }
            Ok(load_purge_job(&mut self.connection, key)
                .await?
                .is_some_and(|job| {
                    job.state != StoredPurgeState::Prepared
                        && job.trash_revision == Some(trash_revision)
                }))
        })
    }

    pub(super) fn claim_due_purge_job(&mut self) -> Result<Option<StoredPurgeJob>> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let mut transaction = self.connection.begin_with("BEGIN IMMEDIATE").await?;
            let key = sqlx::query(
                "SELECT owner_digest, job_id
                   FROM purge_jobs
                  WHERE state = ?1 AND next_attempt_at_ms <= ?2
                  ORDER BY next_attempt_at_ms, created_at_ms, owner_digest, job_id
                  LIMIT 1",
            )
            .bind(PURGE_READY)
            .bind(now)
            .try_map(|row| Ok((row.try_get::<Vec<u8>, _>(0)?, row.try_get::<Vec<u8>, _>(1)?)))
            .fetch_optional(&mut *transaction)
            .await?
            .map(|(owner, id)| purge_key_from_database(owner, id))
            .transpose()?;
            let Some(key) = key else {
                transaction.commit().await?;
                return Ok(None);
            };
            let changed = sqlx::query(
                "UPDATE purge_jobs SET state = ?1, updated_at_ms = ?2
              WHERE owner_digest = ?3 AND job_id = ?4 AND state = ?5",
            )
            .bind(PURGE_CLAIMED)
            .bind(now)
            .bind(key.owner.as_slice())
            .bind(key.id.as_slice())
            .bind(PURGE_READY)
            .execute(&mut *transaction)
            .await
            .map(|result| result.rows_affected())?;
            ensure!(changed == 1, "A selected purge job could not be claimed");
            let job = load_purge_job(&mut transaction, key)
                .await?
                .ok_or_else(|| anyhow!("A claimed purge job disappeared"))?;
            transaction.commit().await?;
            Ok(Some(job))
        })
    }

    pub(super) fn retry_purge_job(&mut self, key: PurgeJobKey, delay_ms: i64) -> Result<bool> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let next_attempt = expiration_time(now, delay_ms)?;
            Ok(sqlx::query(
                "UPDATE purge_jobs
                SET state = ?1,
                    attempts = MIN(attempts + 1, 4294967295),
                    next_attempt_at_ms = ?2,
                    updated_at_ms = ?3
              WHERE owner_digest = ?4 AND job_id = ?5 AND state = ?6",
            )
            .bind(PURGE_READY)
            .bind(next_attempt)
            .bind(now)
            .bind(key.owner.as_slice())
            .bind(key.id.as_slice())
            .bind(PURGE_CLAIMED)
            .execute(&mut self.connection)
            .await
            .map(|result| result.rows_affected())?
                == 1)
        })
    }

    pub(super) fn complete_purge_job(&mut self, key: PurgeJobKey) -> Result<bool> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            Ok(sqlx::query(
                "DELETE FROM purge_jobs
              WHERE owner_digest = ?1 AND job_id = ?2 AND state = ?3",
            )
            .bind(key.owner.as_slice())
            .bind(key.id.as_slice())
            .bind(PURGE_CLAIMED)
            .execute(&mut self.connection)
            .await
            .map(|result| result.rows_affected())?
                == 1)
        })
    }

    pub(super) fn remove_purge_job(&mut self, key: PurgeJobKey) -> Result<bool> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            Ok(
                sqlx::query("DELETE FROM purge_jobs WHERE owner_digest = ?1 AND job_id = ?2")
                    .bind(key.owner.as_slice())
                    .bind(key.id.as_slice())
                    .execute(&mut self.connection)
                    .await
                    .map(|result| result.rows_affected())?
                    == 1,
            )
        })
    }
}

struct StatePathDatabaseRow {
    kind: i64,
    owner: Vec<u8>,
    id: Vec<u8>,
    slot: i64,
    path: Vec<u8>,
    allows_exact_replacement: i64,
}

fn read_state_path_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<StatePathDatabaseRow> {
    Ok(StatePathDatabaseRow {
        kind: row.try_get(0)?,
        owner: row.try_get(1)?,
        id: row.try_get(2)?,
        slot: row.try_get(3)?,
        path: row.try_get(4)?,
        allows_exact_replacement: row.try_get(5)?,
    })
}

fn decode_state_path_row(
    row: StatePathDatabaseRow,
) -> Result<(StatePathCursor, StateBlockingPath)> {
    ensure!(matches!(row.kind, 0 | 1), "State path kind is invalid");
    ensure!(matches!(row.slot, 0 | 1), "State path slot is invalid");
    ensure!(
        matches!(row.allows_exact_replacement, 0 | 1),
        "State path replacement policy is invalid"
    );
    let path = PathBuf::from(OsString::from_vec(row.path));
    validate_stored_path(&path, "State path")?;
    Ok((
        StatePathCursor {
            kind: row.kind,
            owner: row
                .owner
                .try_into()
                .map_err(|_| anyhow!("State path owner digest has an invalid length"))?,
            id: row
                .id
                .try_into()
                .map_err(|_| anyhow!("State path id has an invalid length"))?,
            slot: row.slot,
        },
        StateBlockingPath {
            path,
            allows_exact_replacement: row.allows_exact_replacement != 0,
        },
    ))
}

async fn query_state_blocking_paths(
    connection: &mut Connection,
    after: Option<StatePathCursor>,
    limit: i64,
) -> Result<StatePathPage> {
    const FIRST_PAGE: &str = r#"
WITH state_paths(kind, owner_digest, item_id, slot, path, allows_exact_replacement) AS (
    SELECT 0, owner_digest, upload_id, 0, target_path, state = ?1
      FROM upload_sessions
     WHERE state IN (?1, ?2, ?3)
    UNION ALL
    SELECT 0, owner_digest, upload_id, 1, stage_path, 0
      FROM upload_sessions
     WHERE state IN (?1, ?2, ?3)
    UNION ALL
    SELECT 1, owner_digest, job_id, 0, target_path, 0
      FROM purge_jobs
     WHERE state = ?4
    UNION ALL
    SELECT 1, owner_digest, job_id, 1, trash_path, 0
      FROM purge_jobs
)
SELECT kind, owner_digest, item_id, slot, path, allows_exact_replacement
  FROM state_paths
 ORDER BY kind, owner_digest, item_id, slot
 LIMIT ?5
"#;
    const NEXT_PAGE: &str = r#"
WITH state_paths(kind, owner_digest, item_id, slot, path, allows_exact_replacement) AS (
    SELECT 0, owner_digest, upload_id, 0, target_path, state = ?1
      FROM upload_sessions
     WHERE state IN (?1, ?2, ?3)
    UNION ALL
    SELECT 0, owner_digest, upload_id, 1, stage_path, 0
      FROM upload_sessions
     WHERE state IN (?1, ?2, ?3)
    UNION ALL
    SELECT 1, owner_digest, job_id, 0, target_path, 0
      FROM purge_jobs
     WHERE state = ?4
    UNION ALL
    SELECT 1, owner_digest, job_id, 1, trash_path, 0
      FROM purge_jobs
)
SELECT kind, owner_digest, item_id, slot, path, allows_exact_replacement
  FROM state_paths
 WHERE (kind, owner_digest, item_id, slot) > (?5, ?6, ?7, ?8)
 ORDER BY kind, owner_digest, item_id, slot
 LIMIT ?9
"#;

    let rows = match after {
        Some(after) => {
            sqlx::query(NEXT_PAGE)
                .bind(UPLOAD_RUNNING)
                .bind(UPLOAD_COMMIT_STARTED)
                .bind(UPLOAD_AWAITING_CONFIRMATION)
                .bind(PURGE_PREPARED)
                .bind(after.kind)
                .bind(after.owner.to_vec())
                .bind(after.id.to_vec())
                .bind(after.slot)
                .bind(limit)
                .try_map(|row| read_state_path_row(&row))
                .fetch_all(&mut *connection)
                .await?
        }
        None => {
            sqlx::query(FIRST_PAGE)
                .bind(UPLOAD_RUNNING)
                .bind(UPLOAD_COMMIT_STARTED)
                .bind(UPLOAD_AWAITING_CONFIRMATION)
                .bind(PURGE_PREPARED)
                .bind(limit)
                .try_map(|row| read_state_path_row(&row))
                .fetch_all(&mut *connection)
                .await?
        }
    };
    let decoded = rows
        .into_iter()
        .map(decode_state_path_row)
        .collect::<Result<Vec<_>>>()?;
    let next = (decoded.len() == usize::try_from(limit)?)
        .then(|| decoded.last().map(|(cursor, _)| *cursor))
        .flatten();
    let paths = decoded.into_iter().map(|(_, path)| path).collect();
    Ok(StatePathPage { paths, next })
}

struct PurgeDatabaseRow {
    owner: Vec<u8>,
    id: Vec<u8>,
    target_path: Vec<u8>,
    trash_path: Vec<u8>,
    source_device: Vec<u8>,
    source_inode: Vec<u8>,
    trash_revision: Option<Vec<u8>>,
    is_directory: i64,
    state: i64,
    attempts: i64,
}

fn read_purge_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<PurgeDatabaseRow> {
    Ok(PurgeDatabaseRow {
        owner: row.try_get(0)?,
        id: row.try_get(1)?,
        target_path: row.try_get(2)?,
        trash_path: row.try_get(3)?,
        source_device: row.try_get(4)?,
        source_inode: row.try_get(5)?,
        trash_revision: row.try_get(6)?,
        is_directory: row.try_get(7)?,
        state: row.try_get(8)?,
        attempts: row.try_get(9)?,
    })
}

fn purge_key_from_database(owner: Vec<u8>, id: Vec<u8>) -> Result<PurgeJobKey> {
    Ok(PurgeJobKey {
        owner: owner
            .try_into()
            .map_err(|_| anyhow!("Purge owner digest has an invalid length"))?,
        id: id
            .try_into()
            .map_err(|_| anyhow!("Purge job id has an invalid length"))?,
    })
}

fn decode_purge_row(row: PurgeDatabaseRow) -> Result<StoredPurgeJob> {
    ensure!(
        matches!(row.is_directory, 0 | 1),
        "Purge directory flag is invalid"
    );
    let job = StoredPurgeJob {
        key: purge_key_from_database(row.owner, row.id)?,
        target_path: PathBuf::from(OsString::from_vec(row.target_path)),
        trash_path: PathBuf::from(OsString::from_vec(row.trash_path)),
        source_identity: StoredFileIdentity {
            device: u64::from_be_bytes(
                row.source_device
                    .try_into()
                    .map_err(|_| anyhow!("Purge source device has an invalid length"))?,
            ),
            inode: u64::from_be_bytes(
                row.source_inode
                    .try_into()
                    .map_err(|_| anyhow!("Purge source inode has an invalid length"))?,
            ),
        },
        trash_revision: row
            .trash_revision
            .map(|revision| {
                revision
                    .try_into()
                    .map_err(|_| anyhow!("Purge trash revision has an invalid length"))
            })
            .transpose()?,
        is_directory: row.is_directory == 1,
        state: StoredPurgeState::from_database(row.state)?,
        attempts: u32::try_from(row.attempts)
            .context("Purge attempts in the state database are invalid")?,
    };
    validate_stored_path(&job.target_path, "Purge target")?;
    validate_stored_path(&job.trash_path, "Purge trash")?;
    ensure!(
        job.target_path != job.trash_path,
        "Purge target and trash paths must differ"
    );
    Ok(job)
}

pub(super) async fn load_purge_job(
    connection: &mut Connection,
    key: PurgeJobKey,
) -> Result<Option<StoredPurgeJob>> {
    sqlx::query(
        "SELECT owner_digest, job_id, target_path, trash_path,
                    source_device_be, source_inode_be, trash_revision,
                    is_directory, state, attempts
               FROM purge_jobs
              WHERE owner_digest = ?1 AND job_id = ?2",
    )
    .bind(key.owner.as_slice())
    .bind(key.id.as_slice())
    .try_map(|row| read_purge_row(&row))
    .fetch_optional(&mut *connection)
    .await?
    .map(decode_purge_row)
    .transpose()
}

async fn query_purge_jobs(
    connection: &mut Connection,
    sql: &'static str,
    parameters: sqlx::sqlite::SqliteArguments,
) -> Result<Vec<StoredPurgeJob>> {
    let rows = sqlx::query_with(sql, parameters)
        .try_map(|row| read_purge_row(&row))
        .fetch_all(connection)
        .await?;
    rows.into_iter().map(decode_purge_row).collect()
}
