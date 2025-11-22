// src/ui/settings.rs

use iced::widget::{button, column, container, row, text, slider, text_input};
use iced::{Element, Length, Alignment};

use crate::models::{Settings, Account};
use crate::ui::{view_account, palette, card_style, section_header};

pub struct SettingsView {
    settings: Settings,
    accounts: Vec<Account>,
    new_account_name: String,
    new_ics_url: String,
}

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    VolumeChanged(f32),
    VideoAlertOffsetChanged(f32),
    RegularAlertOffsetChanged(f32),
    SnoozeIntervalChanged(f32),
    MaxSnoozesChanged(f32),
    SyncIntervalChanged(f32),
    AutoJoinToggled(bool),
    ThemeChanged(String),
    AccountNameChanged(String),
    IcsUrlChanged(String),
    AddProtonAccount,
    RemoveAccount(i64),
    SaveSettings,
    TestAudio,
}

impl SettingsView {
    pub fn new() -> Self {
        Self {
            settings: Settings::default(),
            accounts: Vec::new(),
            new_account_name: String::new(),
            new_ics_url: String::new(),
        }
    }

    pub fn update(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::VolumeChanged(v) => self.settings.volume = v,
            SettingsMessage::VideoAlertOffsetChanged(v) => self.settings.video_alert_offset = v as i32,
            SettingsMessage::RegularAlertOffsetChanged(v) => self.settings.regular_alert_offset = v as i32,
            SettingsMessage::SnoozeIntervalChanged(v) => self.settings.snooze_interval = v as i32,
            SettingsMessage::MaxSnoozesChanged(v) => self.settings.max_snoozes = v as i32,
            SettingsMessage::SyncIntervalChanged(v) => self.settings.sync_interval = v as i32,
            SettingsMessage::AutoJoinToggled(v) => self.settings.auto_join_enabled = v,
            SettingsMessage::ThemeChanged(v) => self.settings.theme = v,
            SettingsMessage::AccountNameChanged(v) => self.new_account_name = v,
            SettingsMessage::IcsUrlChanged(v) => self.new_ics_url = v,
            SettingsMessage::AddProtonAccount => {},
            SettingsMessage::RemoveAccount(id) => { self.accounts.retain(|acc| acc.id != Some(id)); },
            SettingsMessage::SaveSettings => {},
            SettingsMessage::TestAudio => {},
        }
    }

    pub fn view(&self) -> Element<SettingsMessage> {
        // Helper for a slider row
        let setting_slider = |label: &str, value: f32, range, msg_fn: fn(f32) -> SettingsMessage, value_text: String| {
            column![
                row![
                    text(label).style(iced::theme::Text::Color(iced::Color::from_rgb(0.2, 0.2, 0.2))),
                    text(value_text).style(iced::theme::Text::Color(iced::Color::from_rgb(0.45, 0.55, 0.50))).size(14),
                ].width(Length::Fill).align_items(Alignment::Center).spacing(10),
                slider(range, value, msg_fn)
                    .step(1.0)
                    .style(iced::theme::Slider::Default), // Iced default is decent, custom requires more boilerplate
            ]
            .spacing(5)
        };

        let audio_section = container(
            column![
                section_header("Audio"),
                setting_slider(
                    "Volume", 
                    self.settings.volume, 
                    0.0..=1.0, 
                    SettingsMessage::VolumeChanged, 
                    format!("{:.0}%", self.settings.volume * 100.0)
                ),
                button("Test Audio Chime")
                    .on_press(SettingsMessage::TestAudio)
                    .style(iced::theme::Button::Secondary)
                    .padding([8, 16]),
            ].spacing(20)
        ).style(card_style).padding(20).width(Length::Fill);

        let alert_section = container(
            column![
                section_header("Notifications"),
                setting_slider(
                    "Video Meeting Alert", 
                    self.settings.video_alert_offset as f32, 
                    1.0..=30.0, 
                    SettingsMessage::VideoAlertOffsetChanged, 
                    format!("{} min before", self.settings.video_alert_offset)
                ),
                setting_slider(
                    "Regular Meeting Alert", 
                    self.settings.regular_alert_offset as f32, 
                    1.0..=30.0, 
                    SettingsMessage::RegularAlertOffsetChanged, 
                    format!("{} min before", self.settings.regular_alert_offset)
                ),
                setting_slider(
                    "Snooze Duration", 
                    self.settings.snooze_interval as f32, 
                    1.0..=30.0, 
                    SettingsMessage::SnoozeIntervalChanged, 
                    format!("{} min", self.settings.snooze_interval)
                ),
            ].spacing(20)
        ).style(card_style).padding(20).width(Length::Fill);

        let account_inputs = column![
            text("Add New Account").style(iced::theme::Text::Color(iced::Color::from_rgb(0.2, 0.2, 0.2))),
            text_input("Account Name", &self.new_account_name)
                .on_input(SettingsMessage::AccountNameChanged)
                .padding(10),
            text_input("ICS URL", &self.new_ics_url)
                .on_input(SettingsMessage::IcsUrlChanged)
                .padding(10),
            button("Connect Account")
                .on_press(SettingsMessage::AddProtonAccount)
                .padding([10, 20])
                .style(iced::theme::Button::Primary),
        ].spacing(10);

        let existing_accounts_list = column(
            self.accounts.iter().map(|acc| {
                row![
                    view_account(acc),
                    button(text("×").size(20))
                        .on_press(SettingsMessage::RemoveAccount(acc.id.unwrap_or(0)))
                        .style(iced::theme::Button::Text)
                        .padding(10)
                ]
                .align_items(Alignment::Center)
                .spacing(10)
                .into()
            }).collect()
        ).spacing(10);

        let accounts_section = container(
            column![
                section_header("Accounts"),
                existing_accounts_list,
                iced::widget::horizontal_rule(10),
                account_inputs,
            ].spacing(20)
        ).style(card_style).padding(20).width(Length::Fill);

        column![
            audio_section,
            alert_section,
            accounts_section,
            button("Save All Changes")
                .on_press(SettingsMessage::SaveSettings)
                .width(Length::Fill)
                .padding(15)
                .style(iced::theme::Button::Primary),
        ]
        .spacing(20)
        .into()
    }

    pub fn set_settings(&mut self, settings: Settings) { self.settings = settings; }
    pub fn set_accounts(&mut self, accounts: Vec<Account>) { self.accounts = accounts; }
}
