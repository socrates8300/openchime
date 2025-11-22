// src/ui/alerts.rs

use iced::widget::{button, column, container, row, text, scrollable};
use iced::{Element, Length, Alignment};

use crate::models::{AlertInfo, AlertType};
use crate::ui::{palette, card_style, section_header};

pub struct AlertsView {
    active_alerts: Vec<AlertInfo>,
    alert_history: Vec<AlertInfo>,
}

#[derive(Debug, Clone)]
pub enum AlertsMessage {
    SnoozeAlert(String),
    DismissAlert(String),
    ClearHistory,
    TestAlert(AlertType),
}

impl AlertsView {
    pub fn new() -> Self {
        Self {
            active_alerts: Vec::new(),
            alert_history: Vec::new(),
        }
    }

    pub fn update(&mut self, message: AlertsMessage) {
        match message {
            AlertsMessage::SnoozeAlert(_) => {},
            AlertsMessage::DismissAlert(id) => { self.active_alerts.retain(|a| a.event.external_id != id); },
            AlertsMessage::ClearHistory => { self.alert_history.clear(); },
            AlertsMessage::TestAlert(_) => {},
        }
    }

    pub fn view(&self) -> Element<AlertsMessage> {
        // Active Alerts Section
        let active_content = if self.active_alerts.is_empty() {
            container(
                text("All quiet. No pending alerts.")
                    .style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5)))
            )
            .padding(20)
            .into()
        } else {
            column(
                self.active_alerts.iter().map(|alert| self.view_active_card(alert)).collect()
            ).spacing(15).into()
        };

        // History Section
        let history_content = if self.alert_history.is_empty() {
            text("History is empty").size(12).style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))).into()
        } else {
            column(
                self.alert_history.iter().take(10).map(|alert| self.view_history_row(alert)).collect()
            ).spacing(8).into()
        };

        // Test Controls (Subtle)
        let test_controls = row![
            button("Test Video").on_press(AlertsMessage::TestAlert(AlertType::VideoMeeting)).style(iced::theme::Button::Text),
            button("Test Regular").on_press(AlertsMessage::TestAlert(AlertType::Meeting)).style(iced::theme::Button::Text),
        ].spacing(10);

        scrollable(
            column![
                section_header("Active Alerts"),
                active_content,
                
                iced::widget::vertical_space(),
                
                row![
                    section_header("Recent History"),
                    iced::widget::horizontal_space(),
                    button("Clear").on_press(AlertsMessage::ClearHistory).style(iced::theme::Button::Text),
                ].align_items(Alignment::Center),
                
                history_content,

                iced::widget::vertical_space(),
                test_controls,
            ]
            .spacing(10)
            .padding(20)
        )
        .into()
    }

    fn view_active_card(&self, alert: &AlertInfo) -> Element<AlertsMessage> {
        let (icon, label) = match alert.alert_type {
            AlertType::VideoMeeting => ("📹", "Video Meeting"),
            AlertType::Meeting => ("📅", "Meeting"),
            _ => ("🔔", "Alert"),
        };

        container(
            column![
                row![
                    text(icon).size(24),
                    column![
                        text(label).size(12).style(palette::ACCENT),
                        text(&alert.event.title).size(18).style(palette::TEXT_MAIN),
                    ].spacing(2),
                ].spacing(15).align_items(Alignment::Center),

                text(format!("Starts in {} minutes", alert.minutes_remaining))
                    .size(14)
                    .style(palette::TEXT_MAIN),

                row![
                    button("Snooze 5m")
                        .on_press(AlertsMessage::SnoozeAlert(alert.event.external_id.clone()))
                        .style(iced::theme::Button::Secondary)
                        .padding([8, 16]),
                    button("Dismiss")
                        .on_press(AlertsMessage::DismissAlert(alert.event.external_id.clone()))
                        .style(iced::theme::Button::Destructive)
                        .padding([8, 16]),
                ].spacing(10).width(Length::Fill).align_items(Alignment::Center)
            ]
            .spacing(15)
        )
        .style(card_style)
        .padding(20)
        .width(Length::Fill)
        .into()
    }

    fn view_history_row(&self, alert: &AlertInfo) -> Element<AlertsMessage> {
        row![
            text("•").style(palette::TEXT_MUTED),
            text(&alert.event.title).width(Length::Fill).style(palette::TEXT_MUTED),
            text(format!("{}m ago", alert.minutes_remaining.abs())).size(12).style(palette::TEXT_MUTED),
        ]
        .spacing(10)
        .align_items(Alignment::Center)
        .into()
    }

    pub fn add_alert(&mut self, alert: AlertInfo) {
        self.active_alerts.push(alert.clone());
        self.alert_history.push(alert);
    }
    pub fn set_active_alerts(&mut self, alerts: Vec<AlertInfo>) { self.active_alerts = alerts; }
}
