// src/ui/calendar.rs

use iced::widget::{button, column, container, row, text, scrollable};
use iced::{Element, Length, Alignment};
use chrono::TimeZone;
use crate::models::CalendarEvent;
use crate::ui::{view_event, palette};

pub struct CalendarView {
    events: Vec<CalendarEvent>,
    selected_date: chrono::NaiveDate,
}

#[derive(Debug, Clone)]
pub enum CalendarMessage {
    DateSelected(chrono::NaiveDate),
    RefreshEvents,
    EventClicked(String),
}

impl CalendarView {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            selected_date: chrono::Local::now().date_naive(),
        }
    }

    pub fn update(&mut self, message: CalendarMessage) {
        match message {
            CalendarMessage::DateSelected(date) => self.selected_date = date,
            CalendarMessage::RefreshEvents => {}, // Logic handled by controller
            CalendarMessage::EventClicked(_) => {}, // Logic handled by controller
        }
    }

    pub fn view(&self) -> Element<CalendarMessage> {
        // 1. Elegant Header
        let date_display = column![
            text(self.selected_date.format("%A").to_string())
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))),
            text(self.selected_date.format("%B %d, %Y").to_string())
                .size(24)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.2, 0.2, 0.2))),
        ].align_items(Alignment::Center);

        let nav_button = |label, msg| {
            button(text(label).size(18))
                .on_press(msg)
                .padding([5, 15])
                .style(iced::theme::Button::Text) // Minimalist buttons
        };

        let header = container(
            row![
                nav_button("‹", CalendarMessage::DateSelected(self.selected_date - chrono::Duration::days(1))),
                date_display,
                nav_button("›", CalendarMessage::DateSelected(self.selected_date + chrono::Duration::days(1))),
            ]
            .spacing(20)
            .align_items(Alignment::Center)
        )
        .padding(20)
        .width(Length::Fill)
        .center_x();

        // 2. Event List
        let mut events_for_day: Vec<_> = self.events
            .iter()
            .filter(|event| {
                // Convert UTC time to local timezone for comparison
                let event_local_date = chrono::Local.from_utc_datetime(&event.start_time.naive_utc()).date_naive();
                event_local_date == self.selected_date
            })
            .collect();
        
        // Ensure events are sorted chronologically within the day
        events_for_day.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        let content = if events_for_day.is_empty() {
            container(
                column![
                    text("No events scheduled").size(18).style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))),
                    text("Enjoy your free time").size(14).style(iced::theme::Text::Color(iced::Color::from_rgb(0.45, 0.55, 0.50))),
                    button("Refresh Calendar")
                        .on_press(CalendarMessage::RefreshEvents)
                        .padding(10)
                        .style(iced::theme::Button::Text)
                ]
                .spacing(10)
                .align_items(Alignment::Center)
            )
            .height(Length::Fill)
            .center_y()
            .center_x()
            .into()
        } else {
            let events: Vec<Element<CalendarMessage>> = events_for_day
                .iter()
                .map(|event| view_event(event).map(|_| CalendarMessage::EventClicked(event.external_id.clone())))
                .collect();

            scrollable(
                column(events)
                    .spacing(15)
                    .padding(20)
            )
            .height(Length::Fill)
            .into()
        };

        column![
            header,
            content,
        ]
        .into()
    }

    pub fn set_events(&mut self, events: Vec<CalendarEvent>) {
        self.events = events;
    }
}
