// OpenChime - Cross-platform meeting reminder app
// Main entry point for iced application

use std::sync::Arc;
use log::{info, error};
use iced::futures::SinkExt; // Import SinkExt for sender.send()

// Helper function to convert technical errors to user-friendly messages
fn user_friendly_error(error: &str) -> String {
    if error.contains("No accounts configured") {
        "Please add a calendar account first in Settings.".to_string()
    } else if error.contains("Failed to fetch accounts") {
        "Could not load your accounts. Please try restarting the app.".to_string()
    } else if error.contains("Failed to sync any accounts") {
        "Could not sync any calendars. Please check your internet connection.".to_string()
    } else if error.contains("network") || error.contains("connection") {
        "Network error. Please check your internet connection and try again.".to_string()
    } else if error.contains("timeout") {
        "Request timed out. Please try again in a moment.".to_string()
    } else if error.contains("Failed to save account") {
        "Could not save account. Please check the account details and try again.".to_string()
    } else if error.contains("Failed to delete account") {
        "Could not delete account. Please try again.".to_string()
    } else if error.contains("Audio test failed") {
        "Could not play audio. Please check your system audio settings.".to_string()
    } else if error.contains("Failed to reload") {
        "Could not refresh data. Please try again.".to_string()
    } else {
        // Fallback: clean up technical error message
        error.replace("Failed to", "Could not")
            .replace("Error:", "")
            .trim()
            .to_string()
    }
}

mod database;
mod models;
mod calendar;
mod alerts;
mod audio;
mod utils;
mod error;
mod ui_state;
mod command_handlers;

use database::Database;
use audio::AudioManager;
use models::{Account, Settings, CalendarEvent};
use ui_state::{UiState, View};
use command_handlers::CommandHandlers;

use iced::widget::{button, column, row, text, text_input, container, scrollable, checkbox};
use iced::{Application, Command, Element, Settings as IcedSettings, Theme, Length};
use log::warn;

// Zen Theme Colors
const ZEN_BG: iced::Color = iced::Color::from_rgb(0.992, 0.988, 0.973); // #FDFCF8
const ZEN_SURFACE: iced::Color = iced::Color::from_rgb(0.949, 0.937, 0.914); // #F2EFE9
const ZEN_TEXT: iced::Color = iced::Color::from_rgb(0.29, 0.29, 0.29); // #4A4A4A
const ZEN_SUBTEXT: iced::Color = iced::Color::from_rgb(0.55, 0.55, 0.55); // #8C8C8C
const ZEN_ACCENT: iced::Color = iced::Color::from_rgb(0.545, 0.616, 0.467); // #8B9D77 (Sage)
const ZEN_ACCENT_HOVER: iced::Color = iced::Color::from_rgb(0.49, 0.56, 0.41);
const ZEN_DESTRUCTIVE: iced::Color = iced::Color::from_rgb(0.831, 0.647, 0.647); // #D4A5A5

// Styles
// (All helper functions removed, using structs below)

/// Unified application message type
/// 
/// This enum handles all message types throughout the application.
/// Messages are organized by domain for better maintainability.
#[derive(Debug, Clone)]
pub enum Message {
    // ===== UI Navigation Messages =====
    /// Switch to calendar view
    ShowCalendar,
    /// Switch to settings view
    ShowSettings,
    /// Switch to alerts view
    ShowAlerts,
    
    // ===== UI Action Messages =====
    /// Trigger manual calendar synchronization
    SyncCalendars,
    /// Test audio system
    TestAudio,
    /// Join meeting with provided URL
    JoinMeeting(String),
    
    // ===== Settings Management Messages =====
    /// Update account name input
    AccountNameChanged(String),
    /// Update ICS URL input
    IcsUrlChanged(String),
    /// Add Proton calendar account
    AddProtonAccount,
    /// Delete account by ID
    DeleteAccount(i64),
    /// Toggle 30-minute alert
    ToggleAlert30m(bool),
    /// Toggle 10-minute alert
    ToggleAlert10m(bool),
    /// Toggle 5-minute alert
    ToggleAlert5m(bool),
    /// Toggle 1-minute alert
    ToggleAlert1m(bool),
    /// Toggle default alert settings
    ToggleAlertDefault(bool),
    
    // ===== Async Operation Result Messages =====
    /// Calendar synchronization completed
    CalendarSyncResult(Result<(), String>),
    /// Audio test completed
    AudioTestResult(Result<(), String>),
    /// Account addition completed
    AccountAdded(Result<Account, String>),
    /// Account deletion completed
    AccountDeleted(Result<(), String>),
    
    // ===== Data Update Messages =====
    /// Events data has been updated
    EventsUpdated(Vec<CalendarEvent>),
    /// Settings data has been updated
    SettingsUpdated(Settings),
    /// Initial data loading completed
    DataLoaded(Vec<CalendarEvent>, Vec<Account>),
    
    // ===== Monitor System Messages =====
    /// Background monitor event received
    MonitorEventReceived(crate::alerts::MonitorEvent),
}

pub struct OpenChimeApp {
    // Core application state
    db: Arc<Database>,
    audio: Arc<AudioManager>,
    
    // Command handlers for async operations
    handlers: CommandHandlers,
    
    // UI state management
    ui_state: UiState,
    
    // Data
    events: Vec<CalendarEvent>,
    settings: Settings,
    accounts: Vec<Account>,
}

// State for alerts module
#[derive(Clone)]
pub struct AppState {
    pub db: std::sync::Arc<Database>,
    pub audio: std::sync::Arc<AudioManager>,
}

impl Application for OpenChimeApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = (Arc<Database>, Arc<AudioManager>);

    fn new((db, audio): Self::Flags) -> (Self, Command<Message>) {
        let handlers = CommandHandlers::new(&db, &audio);
        
        let app = OpenChimeApp {
            db,
            audio,
            handlers,
            ui_state: UiState::new(),
            events: Vec::new(),
            settings: Settings::default(),
            accounts: Vec::new(),
        };
        
        // Load events and accounts on startup
        let db_clone = app.db.clone();
        let startup_command = Command::perform(async move {
            // Load existing events from database
            let events = match sqlx::query_as::<_, crate::models::CalendarEvent>(
                "SELECT id, external_id, account_id, title, description, start_time, end_time, video_link, video_platform, snooze_count, has_alerted, last_alert_threshold, is_dismissed, created_at, updated_at FROM events   ORDER BY start_time ASC LIMIT 50"
            )
            .fetch_all(&db_clone.pool)
            .await {
                Ok(events) => events,
                Err(e) => {
                    log::error!("Failed to load events: {}", e);
                    Vec::new()
                }
            };
            
            // Load accounts
            let accounts = match sqlx::query_as::<_, crate::models::Account>(
                "SELECT id, provider, account_name, auth_data, refresh_token, last_synced_at FROM accounts ORDER BY created_at ASC"
            )
            .fetch_all(&db_clone.pool)
            .await {
                Ok(accounts) => accounts,
                Err(e) => {
                    log::error!("Failed to load accounts: {}", e);
                    Vec::new()
                }
            };
            
            (events, accounts)
        }, |(events, accounts)| Message::DataLoaded(events, accounts));
        
        (app, startup_command)
    }

    fn title(&self) -> String {
        "OpenChime".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ShowCalendar => {
                self.ui_state.current_view = View::Calendar;
                Command::none()
            }
            Message::ShowSettings => {
                self.ui_state.current_view = View::Settings;
                Command::none()
            }
            Message::ShowAlerts => {
                self.ui_state.current_view = View::Alerts;
                Command::none()
            }
            Message::SyncCalendars => {
                self.ui_state.sync_status = "Fetching accounts...".to_string();
                self.ui_state.loading = true;
                let db = self.db.clone();
                Command::perform(async move {
                    // Get all accounts and sync them
                    let accounts = match sqlx::query_as::<_, crate::models::Account>(
                        "SELECT id, provider, account_name, auth_data, refresh_token, last_synced_at FROM accounts"
                    )
                    .fetch_all(&db.pool)
                    .await {
                        Ok(accounts) => {
                            log::info!("Found {} accounts to sync", accounts.len());
                            accounts
                        }
                        Err(e) => return Err(anyhow::anyhow!("Failed to fetch accounts: {}", e))
                    };

                    if accounts.is_empty() {
                        return Err(anyhow::anyhow!("No accounts configured. Please add an account first."));
                    }

                    let mut total_events = 0;
                    let mut successful_syncs = 0;
                    
                    for account in accounts.iter() {
                        log::info!("Attempting to sync account: {} ({})", account.account_name, account.provider);
                        match crate::calendar::sync_account(account, &db.pool).await {
                            Ok(sync_result) => {
                                total_events += sync_result.events_added + sync_result.events_updated;
                                successful_syncs += 1;
                                log::info!("Synced account {}: {} events added, {} events updated", 
                                          account.account_name, sync_result.events_added, sync_result.events_updated);
                            }
                            Err(e) => {
                                log::error!("Failed to sync account {}: {}", account.account_name, e);
                                // Continue with other accounts even if one fails
                            }
                        }
                    }
                    
                    if successful_syncs == 0 {
                        Err(anyhow::anyhow!("Failed to sync any accounts"))
                    } else {
                        log::info!("Sync completed: {} accounts synced, {} total events processed", successful_syncs, total_events);
                        Ok(())
                    }
                }, |result: Result<(), anyhow::Error>| Message::CalendarSyncResult(result.map_err(|e| e.to_string())))
            }
            Message::TestAudio => {
                // Actually test the audio system
                let audio = self.audio.clone();
                Command::perform(async move {
                    match audio.play_alert(crate::audio::AlertType::Meeting) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(anyhow::anyhow!("Audio test failed: {}", e))
                    }
                }, |result: Result<(), anyhow::Error>| Message::AudioTestResult(result.map_err(|e| e.to_string())))
            }
            Message::AccountNameChanged(name) => {
                self.ui_state.account_name = name;
                Command::none()
            }
            Message::IcsUrlChanged(url) => {
                self.ui_state.ics_url = url;
                Command::none()
            }
            Message::AddProtonAccount => {
                if self.ui_state.account_name.is_empty() || self.ui_state.ics_url.is_empty() {
                    return Command::none();
                }
                
                let account = Account::new_proton(self.ui_state.account_name.clone(), self.ui_state.ics_url.clone());
                let db = self.db.clone();
                
                Command::perform(async move {
                    // Actually save the account to database
                    sqlx::query(
                        "INSERT INTO accounts (provider, account_name, auth_data, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
                    )
                    .bind("proton")
                    .bind(&account.account_name)
                    .bind(&account.auth_data)
                    .execute(&db.pool)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to save account: {}", e))?;
                    
                    Ok(account)
                }, |result: Result<Account, anyhow::Error>| Message::AccountAdded(result.map_err(|e| e.to_string())))
            }
            Message::CalendarSyncResult(Ok(())) => {
                self.ui_state.sync_status = "Sync completed successfully".to_string();
                self.ui_state.last_sync_time = Some(chrono::Utc::now());
                self.ui_state.loading = false;
                log::info!("Sync completed successfully, reloading events...");
                // Reload events to show updated data
                let db = self.db.clone();
                Command::perform(async move {
                    sqlx::query_as::<_, crate::models::CalendarEvent>(
                        "SELECT id, external_id, account_id, title, description, start_time, end_time, video_link, video_platform, snooze_count, has_alerted, last_alert_threshold, is_dismissed, created_at, updated_at FROM events   ORDER BY start_time ASC LIMIT 50"
                    )
                    .fetch_all(&db.pool)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to reload events: {}", e))
                }, |result: Result<Vec<CalendarEvent>, anyhow::Error>| {
                    match result {
                        Ok(events) => {
                            log::info!("Reloaded {} events from database", events.len());
                            Message::EventsUpdated(events)
                        }
                        Err(e) => Message::CalendarSyncResult(Err(e.to_string()))
                    }
                })
            }
            Message::CalendarSyncResult(Err(error)) => {
                self.ui_state.sync_status = user_friendly_error(&error);
                self.ui_state.loading = false;
                Command::none()
            }
            Message::AudioTestResult(Ok(())) => {
                info!("Audio test completed successfully");
                Command::none()
            }
            Message::AudioTestResult(Err(error)) => {
                let friendly_error = user_friendly_error(&error);
                self.ui_state.sync_status = friendly_error.clone();
                error!("Audio test failed: {}", error);
                Command::none()
            }
            Message::AccountAdded(Ok(account)) => {
                info!("Account added: {}", account.account_name);
                self.ui_state.account_name.clear();
                self.ui_state.ics_url.clear();
                
                // Reload accounts to show newly added account
                let db = self.db.clone();
                let current_events = self.events.clone();
                
                let reload_accounts = Command::perform(async move {
                    sqlx::query_as::<_, crate::models::Account>(
                        "SELECT id, provider, account_name, auth_data, refresh_token, last_synced_at FROM accounts ORDER BY created_at ASC"
                    )
                    .fetch_all(&db.pool)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to reload accounts: {}", e))
                }, move |result: Result<Vec<Account>, anyhow::Error>| {
                    match result {
                        Ok(accounts) => Message::DataLoaded(current_events.clone(), accounts),
                        Err(e) => Message::AccountAdded(Err(e.to_string()))
                    }
                });

                // Automatically trigger sync to fetch events for the new account
                let trigger_sync = Command::perform(async {}, |_| Message::SyncCalendars);

                Command::batch(vec![reload_accounts, trigger_sync])
            }
            Message::AccountAdded(Err(error)) => {
                let friendly_error = user_friendly_error(&error);
                self.ui_state.sync_status = friendly_error.clone();
                error!("Failed to add account: {}", error);
                Command::none()
            }
            Message::EventsUpdated(events) => {
                log::info!("EventsUpdated received with {} events", events.len());
                self.events = events;
                Command::none()
            }
            Message::SettingsUpdated(settings) => {
                self.settings = settings;
                Command::none()
            }
            Message::DataLoaded(events, accounts) => {
                self.events = events.clone();
                self.accounts = accounts.clone();
                log::info!("Loaded {} events and {} accounts", events.len(), accounts.len());
                
                // Automatically trigger sync to fetch fresh events after loading
                if accounts.len() > 0 {
                    log::info!("Triggering initial calendar sync");
                    self.ui_state.sync_status = "Initial sync...".to_string();
                    self.ui_state.loading = true;
                    Command::perform(async {}, |_| Message::SyncCalendars)
                } else {
                    Command::none()
                }
            }
            Message::DeleteAccount(account_id) => {
                let db = self.db.clone();
                Command::perform(async move {
                    sqlx::query("DELETE FROM accounts WHERE id = ?")
                        .bind(account_id)
                        .execute(&db.pool)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to delete account: {}", e))?;
                    Ok(())
                }, |result: Result<(), anyhow::Error>| Message::AccountDeleted(result.map_err(|e| e.to_string())))
            }
            Message::AccountDeleted(Ok(())) => {
                // Reload accounts to refresh the list
                let db = self.db.clone();
                let current_events = self.events.clone();
                Command::perform(async move {
                    sqlx::query_as::<_, crate::models::Account>(
                        "SELECT id, provider, account_name, auth_data, refresh_token, last_synced_at FROM accounts ORDER BY created_at ASC"
                    )
                    .fetch_all(&db.pool)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to reload accounts: {}", e))
                }, move |result: Result<Vec<Account>, anyhow::Error>| {
                    match result {
                        Ok(accounts) => Message::DataLoaded(current_events.clone(), accounts),
                        Err(e) => Message::AccountDeleted(Err(e.to_string()))
                    }
                })
            }
            Message::AccountDeleted(Err(error)) => {
                let friendly_error = user_friendly_error(&error);
                self.ui_state.sync_status = friendly_error.clone();
                error!("Failed to delete account: {}", error);
                Command::none()
            }
            Message::MonitorEventReceived(event) => {
                match event {
                    crate::alerts::MonitorEvent::AlertTriggered(_calendar_event) => {
                        // Switch to alerts view
                        self.ui_state.current_view = View::Alerts;
                        
                        // Request window attention (flash taskbar/bounce dock)
                        let attention_cmd = iced::window::request_user_attention(iced::window::Id::MAIN, Some(iced::window::UserAttention::Critical));
                        
                        // Reload events to ensure UI shows up-to-date info
                         let db = self.db.clone();
                        let reload_cmd = Command::perform(async move {
                            sqlx::query_as::<_, crate::models::CalendarEvent>(
                                "SELECT id, external_id, account_id, title, description, start_time, end_time, video_link, video_platform, snooze_count, has_alerted, last_alert_threshold, is_dismissed, created_at, updated_at FROM events   ORDER BY start_time ASC LIMIT 50"
                            )
                            .fetch_all(&db.pool)
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to reload events: {}", e))
                        }, |result: Result<Vec<CalendarEvent>, anyhow::Error>| {
                             match result {
                                Ok(events) => Message::EventsUpdated(events),
                                Err(_) => Message::EventsUpdated(Vec::new()) // Ignore error for background refresh
                            }
                        });

                        Command::batch(vec![attention_cmd, reload_cmd])
                    }
                    crate::alerts::MonitorEvent::SyncCompleted { added, updated } => {
                         if added > 0 || updated > 0 {
                            self.ui_state.last_sync_time = Some(chrono::Utc::now());
                            self.ui_state.sync_status = format!("Auto-sync: {} added, {} updated", added, updated);
                            
                            // Refresh events list
                            let db = self.db.clone();
                            Command::perform(async move {
                                sqlx::query_as::<_, crate::models::CalendarEvent>(
                                    "SELECT id, external_id, account_id, title, description, start_time, end_time, video_link, video_platform, snooze_count, has_alerted, last_alert_threshold, is_dismissed, created_at, updated_at FROM events   ORDER BY start_time ASC LIMIT 50"
                                )
                                .fetch_all(&db.pool)
                                .await
                                .map_err(|e| anyhow::anyhow!("Failed to reload events: {}", e))
                            }, |result: Result<Vec<CalendarEvent>, anyhow::Error>| {
                                match result {
                                    Ok(events) => Message::EventsUpdated(events),
                                    Err(_) => Message::EventsUpdated(Vec::new())
                                }
                            })
                        } else {
                             self.ui_state.last_sync_time = Some(chrono::Utc::now());
                             Command::none()
                        }
                    }
                    crate::alerts::MonitorEvent::Error(e) => {
                        log::error!("Background monitor error: {}", e);
                        Command::none()
                    }
                }
            }
            Message::JoinMeeting(url) => {
                log::info!("Opening meeting URL: {}", url);
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&url).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd").arg("/C").arg("start").arg(&url).spawn();
                
                Command::none()
            }
            Message::ToggleAlert30m(enabled) => {
                self.settings.alert_30m = enabled;
                let pool = self.db.pool.clone();
                let settings = self.settings.clone();
                Command::perform(async move {
                    crate::database::settings::update(&pool, &settings).await
                        .map_err(|e| anyhow::anyhow!("Failed to update settings: {}", e))
                }, |res| match res {
                    Ok(_) => Message::SettingsUpdated(Settings::default()), // Dummy message or real update logic? Ideally refetch. For now ignored.
                    Err(e) => Message::CalendarSyncResult(Err(e.to_string())) // Reuse error handler
                })
            }
            Message::ToggleAlert10m(enabled) => {
                self.settings.alert_10m = enabled;
                let pool = self.db.pool.clone();
                let settings = self.settings.clone();
                Command::perform(async move {
                    crate::database::settings::update(&pool, &settings).await
                        .map_err(|e| anyhow::anyhow!("Failed to update settings: {}", e))
                }, |res| match res { Ok(_) => Message::SettingsUpdated(Settings::default()), Err(e) => Message::CalendarSyncResult(Err(e.to_string())) })
            }
            Message::ToggleAlert5m(enabled) => {
                self.settings.alert_5m = enabled;
                let pool = self.db.pool.clone();
                let settings = self.settings.clone();
                Command::perform(async move {
                    crate::database::settings::update(&pool, &settings).await
                        .map_err(|e| anyhow::anyhow!("Failed to update settings: {}", e))
                }, |res| match res { Ok(_) => Message::SettingsUpdated(Settings::default()), Err(e) => Message::CalendarSyncResult(Err(e.to_string())) })
            }
            Message::ToggleAlert1m(enabled) => {
                self.settings.alert_1m = enabled;
                let pool = self.db.pool.clone();
                let settings = self.settings.clone();
                Command::perform(async move {
                    crate::database::settings::update(&pool, &settings).await
                        .map_err(|e| anyhow::anyhow!("Failed to update settings: {}", e))
                }, |res| match res { Ok(_) => Message::SettingsUpdated(Settings::default()), Err(e) => Message::CalendarSyncResult(Err(e.to_string())) })
            }
            Message::ToggleAlertDefault(enabled) => {
                self.settings.alert_default = enabled;
                let pool = self.db.pool.clone();
                let settings = self.settings.clone();
                Command::perform(async move {
                    crate::database::settings::update(&pool, &settings).await
                        .map_err(|e| anyhow::anyhow!("Failed to update settings: {}", e))
                }, |res| match res { Ok(_) => Message::SettingsUpdated(Settings::default()), Err(e) => Message::CalendarSyncResult(Err(e.to_string())) })
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        struct MonitorLoop;
        
        let db = self.db.clone();
        let audio = self.audio.clone();

        iced::subscription::channel(
            std::any::TypeId::of::<MonitorLoop>(),
            100,
            move |mut output| {
                let state = Arc::new(AppState { 
                    db: db.clone(), 
                    audio: audio.clone() 
                });
                
                async move {
                     let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
                     
                     // Spawn the actual monitored logic which defines the sender
                     tokio::spawn(async move {
                         crate::alerts::monitor_meetings(state, Some(sender)).await;
                     });

                     // Forward messages to subscription output
                     loop {
                         if let Some(event) = receiver.recv().await {
                             let _ = output.send(Message::MonitorEventReceived(event)).await;
                         }
                     }
                }
            }
        )
    }

    fn view(&self) -> Element<'_, Message> {
        let nav_button = |label: &str, view: View, current: View, msg: Message| {
            let is_active = view == current;
            button(
                text(label)
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Left)
            )
            .width(Length::Fill)
            .padding(10)
            .style(if is_active {
                iced::theme::Button::Custom(Box::new(ActiveNavStyle))
            } else {
                 iced::theme::Button::Custom(Box::new(NavStyle))
            })
            .on_press(msg)
        };

        let sidebar = container(
            column![
                text("OpenChime")
                    .size(24)
                    .style(iced::theme::Text::Color(ZEN_ACCENT)),
                
                column![
                    nav_button("Calendar", View::Calendar, self.ui_state.current_view.clone(), Message::ShowCalendar),
                    nav_button("Alerts", View::Alerts, self.ui_state.current_view.clone(), Message::ShowAlerts),
                    nav_button("Settings", View::Settings, self.ui_state.current_view.clone(), Message::ShowSettings),
                ]
                .spacing(5),
                
                iced::widget::vertical_space(),
                
                container(
                    column![
                        text("Status")
                            .size(12)
                            .style(iced::theme::Text::Color(ZEN_SUBTEXT)),
                        text(&self.ui_state.sync_status)
                            .size(11)
                            .style(iced::theme::Text::Color(ZEN_TEXT)),
                        text(if let Some(last) = self.ui_state.last_sync_time {
                           format!("Synced: {}", last.format("%H:%M"))
                        } else {
                           "Not synced".to_string()
                        })
                        .size(11)
                        .style(iced::theme::Text::Color(ZEN_SUBTEXT))
                    ]
                    .spacing(4)
                )
                .padding(10)
                .style(iced::theme::Container::Custom(Box::new(CardStyle)))
            ]
            .spacing(40)
            .padding(20)
        )
        .width(200)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(SidebarStyle)));

        let content = container(
            match self.ui_state.current_view {
                View::Calendar => self.view_calendar(),
                View::Settings => self.view_settings(),
                View::Alerts => self.view_alerts(),
            }
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(40);

        container(
            row![
                sidebar,
                content
            ]
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(BackgroundStyle)))
        .into()
    }

    fn theme(&self) -> Theme {
        Theme::Light
    }
}

// Custom Styles implementations
struct ActiveNavStyle;
impl iced::widget::button::StyleSheet for ActiveNavStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(iced::Color::WHITE)),
            text_color: ZEN_ACCENT,
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            shadow: iced::Shadow {
                color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                offset: iced::Vector::new(0.0, 1.0),
                blur_radius: 2.0,
            },
            ..Default::default()
        }
    }
    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        self.active(style)
    }
    fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        self.active(style)
    }
    fn disabled(&self, style: &Self::Style) -> iced::widget::button::Appearance {
         self.active(style)
    }
}

struct NavStyle;
impl iced::widget::button::StyleSheet for NavStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
             background: None,
             text_color: ZEN_TEXT,
             ..Default::default()
        }
    }
    fn hovered(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
         iced::widget::button::Appearance {
             background: Some(iced::Background::Color(iced::Color::from_rgba(0.0,0.0,0.0,0.05))),
             text_color: ZEN_TEXT,
             border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
             },
             ..Default::default()
        }
    }
      fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        self.active(style)
    }
    fn disabled(&self, style: &Self::Style) -> iced::widget::button::Appearance {
         self.active(style)
    }
}

struct SidebarStyle;
impl iced::widget::container::StyleSheet for SidebarStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(ZEN_SURFACE)),
            ..Default::default()
        }
    }
}

struct BackgroundStyle;
impl iced::widget::container::StyleSheet for BackgroundStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(ZEN_BG)),
            ..Default::default()
        }
    }
}

struct CardStyle;
impl iced::widget::container::StyleSheet for CardStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(iced::Color::WHITE)),
            border: iced::Border {
                color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                width: 1.0,
                radius: 12.0.into(),
            },
            shadow: iced::Shadow {
                color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.02),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
            ..Default::default()
        }
    }
}

struct PrimaryButtonStyle;
impl iced::widget::button::StyleSheet for PrimaryButtonStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(ZEN_ACCENT)),
            text_color: iced::Color::WHITE,
            border: iced::Border {
                radius: 6.0.into(),
                 ..Default::default()
            },
             ..Default::default()
        }
    }
    fn hovered(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(ZEN_ACCENT_HOVER)),
            text_color: iced::Color::WHITE,
            border: iced::Border {
                radius: 6.0.into(),
                 ..Default::default()
            },
             ..Default::default()
        }
    }
     fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        self.active(style)
    }
    fn disabled(&self, style: &Self::Style) -> iced::widget::button::Appearance {
         self.active(style)
    }
}

struct DestructiveButtonStyle;
impl iced::widget::button::StyleSheet for DestructiveButtonStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(iced::Color::WHITE)),
            text_color: ZEN_DESTRUCTIVE,
            border: iced::Border {
                color: ZEN_DESTRUCTIVE,
                width: 1.0,
                radius: 6.0.into(),
            },
             ..Default::default()
        }
    }
    fn hovered(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(ZEN_DESTRUCTIVE)),
            text_color: iced::Color::WHITE,
            border: iced::Border {
                color: ZEN_DESTRUCTIVE,
                width: 1.0,
                radius: 6.0.into(),
            },
             ..Default::default()
        }
    }
     fn pressed(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        self.active(style)
    }
    fn disabled(&self, style: &Self::Style) -> iced::widget::button::Appearance {
         self.active(style)
    }
}

impl OpenChimeApp {
    fn view_calendar(&self) -> Element<'_, Message> {
        if self.events.is_empty() {
            container(
                column![
                    text("No upcoming events")
                        .size(24)
                        .style(iced::theme::Text::Color(ZEN_TEXT)),
                    text("Add a calendar account in Settings to get started")
                        .size(16)
                        .style(iced::theme::Text::Color(ZEN_SUBTEXT)),
                    
                    button("Go to Settings")
                        .style(iced::theme::Button::Custom(Box::new(PrimaryButtonStyle)))
                        .padding(12)
                        .on_press(Message::ShowSettings)
                ]
                .spacing(16)
                .align_items(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
        } else {
            // Header with Sync Button
            let header = row![
                text("My Calendar")
                    .size(28)
                    .style(iced::theme::Text::Color(ZEN_TEXT))
                    .width(Length::Fill),
                
                button(if self.ui_state.loading { "Syncing..." } else { "Sync Now" })
                    .style(if self.ui_state.loading { 
                         iced::theme::Button::Custom(Box::new(ActiveNavStyle)) // Greyed look
                    } else {
                         iced::theme::Button::Custom(Box::new(PrimaryButtonStyle))
                    })
                    .padding([8, 16])
                    .on_press(Message::SyncCalendars)
            ]
            .align_items(iced::Alignment::Center);

            // Group events by date
            let mut events_by_date: std::collections::BTreeMap<String, Vec<&CalendarEvent>> = std::collections::BTreeMap::new();
            for event in &self.events {
                let date = event.start_time.format("%Y-%m-%d").to_string();
                events_by_date.entry(date).or_default().push(event);
            }
            
            let mut event_cards = Vec::new();
            
            for (date_str, day_events) in events_by_date {
                // Parse date to show friendly format
                let date_parsed = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap_or_default();
                let friendly_date = date_parsed.format("%A, %B %d").to_string();
                let is_today = date_str == chrono::Utc::now().format("%Y-%m-%d").to_string();

                let date_header = row![
                    text(if is_today { "Today" } else { &friendly_date })
                        .size(18)
                        .style(iced::theme::Text::Color(ZEN_TEXT)),
                    
                    if is_today {
                        text(&friendly_date)
                            .size(14)
                             .style(iced::theme::Text::Color(ZEN_SUBTEXT))
                    } else {
                         text("")
                    }
                ]
                .spacing(10)
                .align_items(iced::Alignment::Center);
                
                let event_rows: Vec<Element<Message>> = day_events.iter().map(|event| {
                    let time_str = event.start_time.format("%I:%M %p").to_string();
                    let is_video = event.video_link.is_some();
                    
                    row![
                        text(time_str)
                            .size(14)
                            .style(iced::theme::Text::Color(ZEN_ACCENT))
                            .width(80),
                        
                        text(if is_video { "📹" } else { "" })
                            .size(16)
                            .width(30),
                            
                        column![
                            text(&event.title)
                                .size(16)
                                .style(iced::theme::Text::Color(ZEN_TEXT)),
                            if let Some(desc) = &event.description {
                                text(desc.lines().next().unwrap_or(""))
                                    .size(12)
                                    .style(iced::theme::Text::Color(ZEN_SUBTEXT))
                            } else {
                                text("")
                            }
                        ]
                    ]
                    .spacing(10)
                    .align_items(iced::Alignment::Center)
                    .padding(8)
                    .into()
                }).collect();
                
                event_cards.push(
                    container(
                        column![
                             date_header,
                             iced::widget::horizontal_rule(1),
                             column(event_rows).spacing(0)
                        ]
                        .spacing(12)
                    )
                    .width(Length::Fill)
                    .padding(20)
                    .style(iced::theme::Container::Custom(Box::new(CardStyle)))
                    .into()
                );
            }
            
            column![
                header,
                scrollable(
                    column(event_cards).spacing(20)
                )
                .height(Length::Fill)
            ]
            .spacing(20)
            .into()
        }
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let accounts_card = container(
            column![
                row![
                    text("Linked Accounts")
                        .size(18)
                        .style(iced::theme::Text::Color(ZEN_TEXT))
                        .width(Length::Fill),
                ],
                
                if self.accounts.is_empty() {
                    Element::from(
                        text("No accounts linked yet.")
                            .style(iced::theme::Text::Color(ZEN_SUBTEXT))
                    )
                } else {
                    column(
                        self.accounts.iter().map(|account| {
                            row![
                                column![
                                     text(&account.account_name)
                                        .size(16)
                                        .style(iced::theme::Text::Color(ZEN_TEXT)),
                                     text(format!("Provider: {}", account.provider))
                                        .size(12)
                                        .style(iced::theme::Text::Color(ZEN_SUBTEXT))
                                ],
                                iced::widget::horizontal_space(),
                                button("Unlink")
                                    .on_press(Message::DeleteAccount(account.id.unwrap_or(0)))
                                    .padding([6, 12])
                                    .style(iced::theme::Button::Custom(Box::new(DestructiveButtonStyle)))
                            ]
                            .align_items(iced::Alignment::Center)
                            .into()
                        }).collect::<Vec<_>>()
                    ).spacing(10).into()
                }
            ]
            .spacing(15)
        )
        .padding(20)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(CardStyle)));

        let add_account_card = container(
            column![
                text("Add New Calendar")
                    .size(18)
                    .style(iced::theme::Text::Color(ZEN_TEXT)),
                
                column![
                    text("Account Label")
                        .size(12)
                        .style(iced::theme::Text::Color(ZEN_SUBTEXT)),
                    text_input("e.g., Work Calendar", &self.ui_state.account_name)
                        .padding(10)
                        .on_input(Message::AccountNameChanged),
                ].spacing(5),

                column![
                    text("ICS Feed URL")
                        .size(12)
                        .style(iced::theme::Text::Color(ZEN_SUBTEXT)),
                    text_input("https://...", &self.ui_state.ics_url)
                        .padding(10)
                        .on_input(Message::IcsUrlChanged),
                ].spacing(5),

                 row![
                    button("Try Sample Feed")
                        .on_press(Message::IcsUrlChanged("https://calendarlabs.com/ical-calendar/ics/48/2025_Events.ics".to_string()))
                        .padding([8, 12])
                        .style(iced::theme::Button::Custom(Box::new(NavStyle))), // Subtle style
                    
                    iced::widget::horizontal_space(),
                    
                    button("Link Account")
                        .on_press(Message::AddProtonAccount)
                        .padding([10, 20])
                        .style(iced::theme::Button::Custom(Box::new(PrimaryButtonStyle))),
                ]
                .align_items(iced::Alignment::Center)
            ]
            .spacing(15)
        )
        .padding(20)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(CardStyle)));
        
         let audio_card = container(
             row![
                column![
                    text("Audio Check")
                        .size(16)
                         .style(iced::theme::Text::Color(ZEN_TEXT)),
                    text("Test your speaker volume")
                         .size(12)
                         .style(iced::theme::Text::Color(ZEN_SUBTEXT)),
                ],
                iced::widget::horizontal_space(),
                button("Play Sound")
                    .on_press(Message::TestAudio)
                    .padding([8, 16])
                    .style(iced::theme::Button::Custom(Box::new(PrimaryButtonStyle)))
             ]
             .align_items(iced::Alignment::Center)
         )
         .padding(20)
         .width(Length::Fill)
         .style(iced::theme::Container::Custom(Box::new(CardStyle)));

        let alerts_card = container(
            column![
                text("Notification Settings")
                    .size(18)
                    .style(iced::theme::Text::Color(ZEN_TEXT)),
                
                checkbox("Alert 30 minutes before", self.settings.alert_30m)
                    .on_toggle(Message::ToggleAlert30m),
                checkbox("Alert 10 minutes before", self.settings.alert_10m)
                    .on_toggle(Message::ToggleAlert10m),
                checkbox("Alert 5 minutes before", self.settings.alert_5m)
                    .on_toggle(Message::ToggleAlert5m),
                checkbox("Alert 1 minute before", self.settings.alert_1m)
                    .on_toggle(Message::ToggleAlert1m),
                checkbox("Alert at start time", self.settings.alert_default)
                    .on_toggle(Message::ToggleAlertDefault),
            ]
            .spacing(15)
        )
        .padding(20)
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(CardStyle)));

        scrollable(
             column![
                text("Settings")
                    .size(28)
                    .style(iced::theme::Text::Color(ZEN_TEXT)),
                accounts_card,
                alerts_card,
                add_account_card,
                audio_card
            ]
            .spacing(20)
        )
        .into()
    }
    
    fn view_alerts(&self) -> Element<'_, Message> {
        // Show upcoming events that need alerts
        let now = chrono::Utc::now();
        let upcoming_events: Vec<_> = self.events.iter()
            .filter(|event| {
                let minutes_until = (event.start_time - now).num_minutes();
                (-5..=60).contains(&minutes_until) // Show active events too
            })
            // Sort primarily by urgency (happening soonest)
            .collect();

        let header = text("Alerts Center")
                .size(28)
                .style(iced::theme::Text::Color(ZEN_TEXT));

        if upcoming_events.is_empty() {
             column![
                header,
                container(
                    column![
                         text("All Clear")
                            .size(24)
                            .style(iced::theme::Text::Color(ZEN_ACCENT)),
                         text("No upcoming meetings in the next hour.")
                            .style(iced::theme::Text::Color(ZEN_SUBTEXT)),
                    ]
                    .align_items(iced::Alignment::Center)
                    .spacing(10)
                )
                .width(Length::Fill)
                .height(300)
                .center_x()
                .center_y()
                .style(iced::theme::Container::Custom(Box::new(CardStyle)))
            ]
            .spacing(20)
            .into()
        } else {
            let alert_cards: Vec<Element<Message>> = upcoming_events.iter().map(|event| {
                let minutes_until = (event.start_time - now).num_minutes();
                let is_video = event.video_link.is_some();
                
                // Dynamic styling based on urgency
                let (urgency_color, urgency_text) = if minutes_until <= 0 {
                     (ZEN_ACCENT, "Now".to_string())
                } else if minutes_until <= 5 {
                     (ZEN_DESTRUCTIVE, format!("In {} min", minutes_until))
                } else {
                     (ZEN_ACCENT, format!("In {} min", minutes_until))
                };

                container(
                    row![
                        // Time Column
                        column![
                             text(urgency_text)
                                 .size(18)
                                 .style(iced::theme::Text::Color(urgency_color))
                                 .width(80),
                             text(event.start_time.format("%H:%M"))
                                 .size(12)
                                 .style(iced::theme::Text::Color(ZEN_SUBTEXT))
                        ],
                        
                        // Divider
                        container("").width(1).height(40).style(iced::theme::Container::Custom(Box::new(CardStyle))), // Hacky vertical divider
                        
                        // Info Column
                        column![
                             text(&event.title)
                                 .size(18)
                                 .style(iced::theme::Text::Color(ZEN_TEXT)),
                             if is_video {
                                 text("Video Meeting Detected")
                                     .size(12)
                                     .style(iced::theme::Text::Color(ZEN_ACCENT))
                             } else {
                                 text("In Person / No Link")
                                     .size(12)
                                     .style(iced::theme::Text::Color(ZEN_SUBTEXT))
                             }
                        ]
                        .padding([0, 10]),
                        
                        iced::widget::horizontal_space(),
                        
                        // Action Button
                        if let Some(url) = &event.video_link {
                             Element::from(button("Join Meeting")
                                .padding([10, 20])
                                .style(iced::theme::Button::Custom(Box::new(PrimaryButtonStyle)))
                                .on_press(Message::JoinMeeting(url.clone())))
                        } else {
                             Element::from(text(""))
                        }
                    ]
                    .align_items(iced::Alignment::Center)
                )
                .width(Length::Fill)
                .padding(20)
                .style(iced::theme::Container::Custom(Box::new(CardStyle)))
                .into()
            }).collect();
            
            column![
                header,
                scrollable(
                    column(alert_cards).spacing(15)
                )
            ]
            .spacing(20)
            .into()
        }
    }
}



#[tokio::main]
async fn main() -> iced::Result {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting OpenChime with iced UI");

    // Initialize core components
    let db = match Database::new().await {
        Ok(database) => Arc::new(database),
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            eprintln!("Failed to initialize database: {}", e);
            eprintln!("Please check your system and try again.");
            std::process::exit(1);
        }
    };
    
    let audio = match AudioManager::new() {
        Ok(audio_manager) => Arc::new(audio_manager),
        Err(e) => {
            warn!("Failed to initialize audio system: {}", e);
            warn!("Continuing without audio - audio features will be disabled");
            // Continue without audio - create a dummy audio manager
            Arc::new(AudioManager::new_dummy())
        }
    };

    // Run iced application
    OpenChimeApp::run(IcedSettings {
        flags: (db, audio),
        window: iced::window::Settings {
            size: iced::Size::new(800.0, 600.0),
            resizable: true,
            ..Default::default()
        },
        id: None,
        fonts: vec![],
        default_font: Default::default(),
        default_text_size: iced::Pixels(16.0),
        antialiasing: false,
    })
}