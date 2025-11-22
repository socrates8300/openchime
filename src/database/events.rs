// file: src/database/events.rs
use anyhow::Result;
use sqlx::SqlitePool;

pub async fn get_upcoming(pool: &SqlitePool) -> Result<Vec<crate::models::CalendarEvent>> {
    let now = chrono::Utc::now();
    let end_of_day = now
        .date_naive()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_utc();

    let events = sqlx::query_as::<_, crate::models::CalendarEvent>(
        r#"
        SELECT 
            id, external_id, account_id, title, description, start_time, end_time,
            video_link, video_platform, snooze_count, has_alerted, last_alert_threshold,
            is_dismissed, created_at, updated_at
        FROM events 
        WHERE start_time >= ? 
            AND start_time <= ?
            AND is_dismissed = 0
        ORDER BY start_time ASC
        "#,
    )
    .bind(now)
    .bind(end_of_day)
    .fetch_all(pool)
    .await?;

    Ok(events)
}

pub async fn get_needing_alert(pool: &SqlitePool) -> Result<Vec<crate::models::CalendarEvent>> {
    let now = chrono::Utc::now();
    let video_threshold = now + chrono::Duration::minutes(3);
    let regular_threshold = now + chrono::Duration::minutes(1);

    let events = sqlx::query_as::<_, crate::models::CalendarEvent>(
        r#"
        SELECT 
            id, external_id, account_id, title, description, start_time, end_time,
            video_link, video_platform, snooze_count, has_alerted, last_alert_threshold,
            is_dismissed, created_at, updated_at
        FROM events 
        WHERE has_alerted = 0 
            AND is_dismissed = 0
            AND (
                (video_link IS NOT NULL AND start_time <= ?)
                OR (video_link IS NULL AND start_time <= ?)
            )
        ORDER BY start_time ASC
        "#,
    )
    .bind(video_threshold)
    .bind(regular_threshold)
    .fetch_all(pool)
    .await?;

    Ok(events)
}

pub async fn mark_alerted(pool: &SqlitePool, event_id: &str) -> Result<()> {
    sqlx::query("UPDATE events SET has_alerted = 1 WHERE id = ?")
        .bind(event_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn snooze(pool: &SqlitePool, event_id: &str) -> Result<()> {
    // Check current snooze count
    let snooze_count: i32 =
        sqlx::query_scalar("SELECT snooze_count FROM events WHERE id = ?")
            .bind(event_id)
            .fetch_one(pool)
            .await?;

    if snooze_count >= 3 {
        return Err(anyhow::anyhow!("Maximum snooze limit reached"));
    }

    // Update snooze count and timestamp
    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE events SET snooze_count = snooze_count + 1, last_snoozed_at = ?, has_alerted = 0 WHERE id = ?"
    )
    .bind(now)
    .bind(event_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn dismiss(pool: &SqlitePool, event_id: &str) -> Result<()> {
    sqlx::query("UPDATE events SET is_dismissed = 1 WHERE id = ?")
        .bind(event_id)
        .execute(pool)
        .await?;

    Ok(())
}
