use super::*;

impl StoreWorker {
    pub(super) fn begin_operation(
        &mut self,
        key: OperationKey,
        fingerprint: [u8; 32],
    ) -> Result<StoreBegin> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let mut transaction = self.connection.begin_with("BEGIN IMMEDIATE").await?;
            purge_expired(&mut transaction, now).await?;

            if let Some(existing) = load_operation(&mut transaction, key).await? {
                let result = if existing.fingerprint != fingerprint {
                    StoreBegin::Conflict
                } else {
                    match existing.state {
                        OPERATION_RESERVED | OPERATION_COMMIT_STARTED => StoreBegin::Running,
                        OPERATION_COMPLETED => StoreBegin::Replay(existing.into_outcome()?),
                        state => bail!("Invalid operation state in the state database: {state}"),
                    }
                };
                transaction.commit().await?;
                return Ok(result);
            }

            let global_count: i64 = sqlx::query("SELECT COUNT(*) FROM operations")
                .try_map(|row: sqlx::sqlite::SqliteRow| row.try_get(0))
                .fetch_one(&mut *transaction)
                .await?;
            let owner_count: i64 =
                sqlx::query("SELECT COUNT(*) FROM operations WHERE owner_digest = ?1")
                    .bind(key.owner.as_slice())
                    .try_map(|row: sqlx::sqlite::SqliteRow| row.try_get(0))
                    .fetch_one(&mut *transaction)
                    .await?;
            if global_count >= self.limits.capacity || owner_count >= self.limits.per_owner {
                transaction.commit().await?;
                return Ok(StoreBegin::Full);
            }

            let lease = Uuid::new_v4().into_bytes();
            sqlx::query(
                "INSERT INTO operations(
                 owner_digest, operation_id, fingerprint, lease_token, state,
                 created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            )
            .bind(key.owner.as_slice())
            .bind(key.id.as_slice())
            .bind(fingerprint.as_slice())
            .bind(lease.as_slice())
            .bind(OPERATION_RESERVED)
            .bind(now)
            .execute(&mut *transaction)
            .await
            .map(|result| result.rows_affected())?;
            transaction.commit().await?;
            Ok(StoreBegin::Started { lease })
        })
    }

    pub(super) fn operation_status(&mut self, key: OperationKey) -> Result<StoreStatus> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let mut transaction = self.connection.begin_with("BEGIN IMMEDIATE").await?;
            purge_expired(&mut transaction, now).await?;
            let result = match load_operation(&mut transaction, key).await? {
                Some(operation) => match operation.state {
                    OPERATION_RESERVED | OPERATION_COMMIT_STARTED => StoreStatus::Running,
                    OPERATION_COMPLETED => StoreStatus::Completed(operation.into_outcome()?),
                    state => bail!("Invalid operation state in the state database: {state}"),
                },
                None => StoreStatus::NotFound,
            };
            transaction.commit().await?;
            Ok(result)
        })
    }

    pub(super) fn mark_operation_commit_started(
        &mut self,
        key: OperationKey,
        lease: [u8; 16],
    ) -> Result<bool> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let mut transaction = self.connection.begin_with("BEGIN IMMEDIATE").await?;
            let changed = sqlx::query(
                "UPDATE operations SET state = ?1, updated_at_ms = ?2
              WHERE owner_digest = ?3
                AND operation_id = ?4
                AND lease_token = ?5
                AND state = ?6",
            )
            .bind(OPERATION_COMMIT_STARTED)
            .bind(now)
            .bind(key.owner.as_slice())
            .bind(key.id.as_slice())
            .bind(lease.as_slice())
            .bind(OPERATION_RESERVED)
            .execute(&mut *transaction)
            .await
            .map(|result| result.rows_affected())?;
            transaction.commit().await?;
            Ok(changed == 1)
        })
    }

    pub(super) fn complete_operation(
        &mut self,
        key: OperationKey,
        lease: [u8; 16],
        outcome: &StoredOutcome,
    ) -> Result<bool> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let expires_at = expiration_time(now, self.limits.ttl_ms)?;
            let mut transaction = self.connection.begin_with("BEGIN IMMEDIATE").await?;
            let changed = sqlx::query(
                "UPDATE operations
                SET state = ?1,
                    terminal_state = ?2,
                    http_status = ?3,
                    error_code = ?4,
                    updated_at_ms = ?5,
                    expires_at_ms = ?6
              WHERE owner_digest = ?7
                AND operation_id = ?8
                AND lease_token = ?9
                AND state IN (?10, ?11)",
            )
            .bind(OPERATION_COMPLETED)
            .bind(outcome.state as i64)
            .bind(i64::from(outcome.status))
            .bind(outcome.code.as_deref())
            .bind(now)
            .bind(expires_at)
            .bind(key.owner.as_slice())
            .bind(key.id.as_slice())
            .bind(lease.as_slice())
            .bind(OPERATION_RESERVED)
            .bind(OPERATION_COMMIT_STARTED)
            .execute(&mut *transaction)
            .await
            .map(|result| result.rows_affected())?;
            transaction.commit().await?;
            Ok(changed == 1)
        })
    }

    pub(super) fn abandon_operation(&mut self, key: OperationKey, lease: [u8; 16]) -> Result<()> {
        xcss_sqlite::block_on_sqlite_connection(async {
            self.reset_deadline().await?;
            let now = self.now_ms()?;
            let expires_at = expiration_time(now, self.limits.ttl_ms)?;
            let mut transaction = self.connection.begin_with("BEGIN IMMEDIATE").await?;
            let removed = sqlx::query(
                "DELETE FROM operations
              WHERE owner_digest = ?1
                AND operation_id = ?2
                AND lease_token = ?3
                AND state = ?4",
            )
            .bind(key.owner.as_slice())
            .bind(key.id.as_slice())
            .bind(lease.as_slice())
            .bind(OPERATION_RESERVED)
            .execute(&mut *transaction)
            .await
            .map(|result| result.rows_affected())?;
            if removed == 0 {
                sqlx::query(
                    "UPDATE operations
                    SET state = ?1,
                        terminal_state = ?2,
                        http_status = ?3,
                        error_code = ?4,
                        updated_at_ms = ?5,
                        expires_at_ms = ?6
                  WHERE owner_digest = ?7
                    AND operation_id = ?8
                    AND lease_token = ?9
                    AND state = ?10",
                )
                .bind(OPERATION_COMPLETED)
                .bind(StoredTerminalState::Unknown as i64)
                .bind(i64::from(UNKNOWN_STATUS))
                .bind(UNKNOWN_CODE)
                .bind(now)
                .bind(expires_at)
                .bind(key.owner.as_slice())
                .bind(key.id.as_slice())
                .bind(lease.as_slice())
                .bind(OPERATION_COMMIT_STARTED)
                .execute(&mut *transaction)
                .await
                .map(|result| result.rows_affected())?;
            }
            transaction.commit().await?;
            Ok(())
        })
    }
}

struct LoadedOperation {
    fingerprint: [u8; 32],
    state: i64,
    terminal_state: Option<i64>,
    http_status: Option<i64>,
    code: Option<String>,
}

impl LoadedOperation {
    fn into_outcome(self) -> Result<StoredOutcome> {
        let state = self
            .terminal_state
            .ok_or_else(|| anyhow!("Completed operation is missing its terminal state"))?;
        let status = self
            .http_status
            .ok_or_else(|| anyhow!("Completed operation is missing its HTTP status"))?;
        let status =
            u16::try_from(status).context("Completed operation contains an invalid HTTP status")?;
        let outcome = StoredOutcome {
            status,
            state: StoredTerminalState::from_database(state)?,
            code: self.code,
        };
        outcome.validate()?;
        Ok(outcome)
    }
}

async fn load_operation(
    connection: &mut Connection,
    key: OperationKey,
) -> Result<Option<LoadedOperation>> {
    let row = sqlx::query(
        "SELECT fingerprint, state, terminal_state, http_status, error_code
               FROM operations
              WHERE owner_digest = ?1 AND operation_id = ?2",
    )
    .bind(key.owner.as_slice())
    .bind(key.id.as_slice())
    .try_map(|row| {
        Ok((
            row.try_get::<Vec<u8>, _>(0)?,
            row.try_get::<i64, _>(1)?,
            row.try_get::<Option<i64>, _>(2)?,
            row.try_get::<Option<i64>, _>(3)?,
            row.try_get::<Option<String>, _>(4)?,
        ))
    })
    .fetch_optional(&mut *connection)
    .await?;
    row.map(|(fingerprint, state, terminal_state, http_status, code)| {
        let fingerprint = <[u8; 32]>::try_from(fingerprint)
            .map_err(|_| anyhow!("Operation fingerprint has an invalid length"))?;
        Ok(LoadedOperation {
            fingerprint,
            state,
            terminal_state,
            http_status,
            code,
        })
    })
    .transpose()
}

pub(super) async fn purge_expired(transaction: &mut Connection, now: i64) -> Result<()> {
    // The partial index has a fixed state predicate. Keep that predicate
    // literal in this statement so SQLite opens the correct table cursor;
    // a parameterized partial-index term can generate an invalid covering
    // DELETE plan in SQLite 3.46.0 (upstream fix 7058d93b09).
    let statement = format!(
        "DELETE FROM operations WHERE state = {OPERATION_COMPLETED} AND expires_at_ms <= ?1"
    );
    sqlx::query(sqlx::AssertSqlSafe(statement.to_owned()))
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map(|result| result.rows_affected())?;
    Ok(())
}
