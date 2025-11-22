// file: src/database/accounts.rs
use anyhow::Result;
use sqlx::SqlitePool;

pub async fn add(pool: &SqlitePool, account: &crate::models::Account) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO accounts (provider, account_name, auth_data, refresh_token) VALUES (?, ?, ?, ?)"
    )
    .bind(&account.provider)
    .bind(&account.account_name)
    .bind(&account.auth_data)
    .bind(&account.refresh_token)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<crate::models::Account>> {
    let accounts = sqlx::query_as::<_, crate::models::Account>(
        "SELECT id, provider, account_name, auth_data, refresh_token, last_synced_at FROM accounts",
    )
    .fetch_all(pool)
    .await?;

    Ok(accounts)
}

pub async fn update_sync_time(pool: &SqlitePool, account_id: i64) -> Result<()> {
    let now = chrono::Utc::now();
    sqlx::query("UPDATE accounts SET last_synced_at = ? WHERE id = ?")
        .bind(now)
        .bind(account_id)
        .execute(pool)
        .await?;

    Ok(())
}
