// file: src/database/accounts.rs
// ICS-only mode - no encryption needed for public ICS URLs
use anyhow::Result;
use sqlx::SqlitePool;

pub async fn add(pool: &SqlitePool, account: &crate::models::Account) -> Result<i64> {
    // ICS URLs stored as plain text - they're public/semi-public links
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
    // ICS URLs retrieved as plain text - no decryption needed
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Account;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create accounts table
        sqlx::query(
            r#"
            CREATE TABLE accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider TEXT NOT NULL,
                account_name TEXT NOT NULL,
                auth_data TEXT NOT NULL,
                refresh_token TEXT,
                last_synced_at DATETIME
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_add_account_stores_plaintext() {
        let pool = setup_test_db().await;
        let account = Account::new_google(
            "test@gmail.com".to_string(),
            "plaintext_auth_data".to_string(),
            Some("plaintext_refresh_token".to_string()),
        );

        // Add account
        let account_id = add(&pool, &account).await.unwrap();
        assert!(account_id > 0);

        // Read directly from database
        let row: (String, Option<String>) = sqlx::query_as(
            "SELECT auth_data, refresh_token FROM accounts WHERE id = ?",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // Verify data is stored as plaintext
        assert_eq!(row.0, "plaintext_auth_data");
        assert_eq!(row.1, Some("plaintext_refresh_token".to_string()));
    }

    #[tokio::test]
    async fn test_get_all_retrieves_data() {
        let pool = setup_test_db().await;
        let account = Account::new_google(
            "test@gmail.com".to_string(),
            "auth_data".to_string(),
            Some("refresh_token".to_string()),
        );

        // Add account
        add(&pool, &account).await.unwrap();

        // Get all accounts
        let accounts = get_all(&pool).await.unwrap();
        assert_eq!(accounts.len(), 1);

        // Verify data is retrieved correctly
        let retrieved = &accounts[0];
        assert_eq!(retrieved.account_name, "test@gmail.com");
        assert_eq!(retrieved.auth_data, "auth_data");
        assert_eq!(
            retrieved.refresh_token,
            Some("refresh_token".to_string())
        );
    }

    #[tokio::test]
    async fn test_proton_account_persistence() {
        let pool = setup_test_db().await;
        let account = Account::new_proton(
            "user@proton.me".to_string(),
            "https://calendar.proton.me/ics/secret".to_string(),
        );

        // Add and retrieve Proton account
        add(&pool, &account).await.unwrap();
        let accounts = get_all(&pool).await.unwrap();
        let retrieved = &accounts[0];

        // Verify data preserved
        assert_eq!(retrieved.account_name, "user@proton.me");
        assert_eq!(
            retrieved.auth_data,
            "https://calendar.proton.me/ics/secret"
        );
        assert_eq!(retrieved.refresh_token, None);
    }
}
