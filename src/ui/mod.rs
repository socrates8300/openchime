// src/ui/mod.rs

use iced::widget::{column, container, row, text};
use iced::{Element, Color, Background, Border, Shadow, Vector, Theme};

use crate::models::{CalendarEvent, Account};


pub mod styles;

// --- ZEN THEME PALETTE ---
pub mod palette {
    use iced::Color;

    pub const BACKGROUND: Color = Color::from_rgb(0.98, 0.97, 0.95); // Warm Sand
    pub const SURFACE: Color = Color::WHITE;
    pub const TEXT_MAIN: Color = Color::from_rgb(0.2, 0.2, 0.2);     // Soft Charcoal
    pub const TEXT_MUTED: Color = Color::from_rgb(0.5, 0.5, 0.5);
    pub const ACCENT: Color = Color::from_rgb(0.45, 0.55, 0.50);     // Sage Green
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.35, 0.45, 0.40);
    pub const DANGER: Color = Color::from_rgb(0.8, 0.4, 0.4);        // Muted Red
}

// --- REUSABLE STYLES ---

pub fn card_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(palette::SURFACE)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 10.0,
        },
        text_color: Some(palette::TEXT_MAIN),
    }
}

// --- COMPONENT VIEWS ---

pub fn view_event(event: &CalendarEvent) -> Element<'_, crate::messages::Message> {
    let is_video = event.is_video_meeting();

    let icon = if is_video { "📹" } else { "📅" };

    // Convert UTC times to local timezone for display
    let local_start = event.start_time.with_timezone(&chrono::Local);
    let local_end = event.end_time.with_timezone(&chrono::Local);

    let time_str = format!(
        "{} - {}",
        local_start.format("%H:%M"),
        local_end.format("%H:%M")
    );

    // A visual strip on the left to indicate event type
    let indicator_color = if is_video { palette::ACCENT } else { palette::TEXT_MUTED };

    container(
        row![
            // Colored strip
            container("").width(4).height(40).style(container::Appearance {
                background: Some(Background::Color(indicator_color)),
                border: Border { radius: 2.0.into(), ..Border::default() },
                ..Default::default()
            }),
            
            // Content
            column![
                text(&event.title).size(16).style(palette::TEXT_MAIN),
                row![
                    text(icon).size(14),
                    text(time_str).size(14).style(palette::TEXT_MUTED),
                ].spacing(6)
            ].spacing(4)
        ]
        .spacing(12)
        .align_items(iced::Alignment::Center)
    )
    .padding(15)
    .style(card_style)
    .into()
}

pub fn view_account(account: &Account) -> Element<'_, crate::messages::Message> {
    container(
        column![
            row![
                text(&account.account_name).size(16).style(palette::TEXT_MAIN),
                status_badge("Active", true),
            ].spacing(10).align_items(iced::Alignment::Center),
            
            text(format!("Provider: {}", account.provider))
                .size(12)
                .style(palette::TEXT_MUTED),
        ]
        .spacing(5)
    )
    .padding(15)
    .width(iced::Length::Fill)
    .style(card_style)
    .into()
}

pub fn status_badge(label: &str, is_positive: bool) -> Element<'_, crate::messages::Message> {
    let (bg, text_color) = if is_positive {
        (Color::from_rgba(0.45, 0.55, 0.50, 0.2), palette::ACCENT) // Sage tint
    } else {
        (Color::from_rgba(0.5, 0.5, 0.5, 0.1), palette::TEXT_MUTED)
    };

    container(text(label).size(10).style(iced::theme::Text::Color(text_color)))
        .padding([4, 8])
        .style(container::Appearance {
            background: Some(Background::Color(bg)),
            border: Border { radius: 10.0.into(), ..Border::default() },
            ..Default::default()
        })
        .into()
}

// Helper for section headers
pub fn section_header(label: &str) -> Element<'_, crate::messages::Message> {
    text(label)
        .size(20)
        .style(palette::ACCENT)
        .into()
}
