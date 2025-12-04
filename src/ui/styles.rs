use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

// Zen Theme Colors
pub const ZEN_BG: Color = Color::from_rgb(0.992, 0.988, 0.973); // #FDFCF8
pub const ZEN_SURFACE: Color = Color::from_rgb(0.949, 0.937, 0.914); // #F2EFE9
pub const ZEN_TEXT: Color = Color::from_rgb(0.29, 0.29, 0.29); // #4A4A4A
pub const ZEN_SUBTEXT: Color = Color::from_rgb(0.55, 0.55, 0.55); // #8C8C8C
pub const ZEN_ACCENT: Color = Color::from_rgb(0.545, 0.616, 0.467); // #8B9D77 (Sage)
pub const ZEN_ACCENT_HOVER: Color = Color::from_rgb(0.49, 0.56, 0.41);
pub const ZEN_DESTRUCTIVE: Color = Color::from_rgb(0.831, 0.647, 0.647); // #D4A5A5

pub struct ActiveNavStyle;
impl button::StyleSheet for ActiveNavStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::WHITE)),
            text_color: ZEN_ACCENT,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 2.0,
            },
            ..Default::default()
        }
    }
    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
    fn disabled(&self, style: &Self::Style) -> button::Appearance {
         self.active(style)
    }
}

pub struct NavStyle;
impl button::StyleSheet for NavStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: ZEN_SUBTEXT,
            border: Border::default(),
            shadow: Shadow::default(),
            ..Default::default()
        }
    }
    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.03))),
            text_color: ZEN_TEXT,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            shadow: Shadow::default(),
            ..Default::default()
        }
    }
    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.hovered(style)
    }
    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
}

pub struct SidebarStyle;
impl container::StyleSheet for SidebarStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(ZEN_SURFACE)),
            border: Border {
                width: 1.0,
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub struct BackgroundStyle;
impl container::StyleSheet for BackgroundStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(ZEN_BG)),
            ..Default::default()
        }
    }
}

pub struct CardStyle;
impl container::StyleSheet for CardStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::WHITE)),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.03),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.02),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 4.0,
            },
            ..Default::default()
        }
    }
}

pub struct InputStyle;
impl iced::widget::text_input::StyleSheet for InputStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: Background::Color(Color::WHITE),
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
            },
            icon_color: ZEN_SUBTEXT,
        }
    }
    fn focused(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: Background::Color(Color::WHITE),
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: ZEN_ACCENT,
            },
            icon_color: ZEN_ACCENT,
        }
    }
    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgba(0.0, 0.0, 0.0, 0.3)
    }
    fn value_color(&self, _style: &Self::Style) -> Color {
        ZEN_TEXT
    }
    fn disabled_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgba(0.0, 0.0, 0.0, 0.3)
    }
    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::from_rgba(0.545, 0.616, 0.467, 0.2)
    }
    fn disabled(&self, style: &Self::Style) -> iced::widget::text_input::Appearance {
        self.active(style)
    }
}

pub struct PrimaryButtonStyle;
impl button::StyleSheet for PrimaryButtonStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(ZEN_ACCENT)),
            text_color: Color::WHITE,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 4.0,
            },
            ..Default::default()
        }
    }
    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(ZEN_ACCENT_HOVER)),
            text_color: Color::WHITE,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
                offset: Vector::new(0.0, 3.0),
                blur_radius: 5.0,
            },
            ..Default::default()
        }
    }
    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
    fn disabled(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.8, 0.8, 0.8))),
            text_color: Color::from_rgb(0.5, 0.5, 0.5),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub struct DestructiveButtonStyle;
impl button::StyleSheet for DestructiveButtonStyle {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: ZEN_DESTRUCTIVE,
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: ZEN_DESTRUCTIVE,
            },
            ..Default::default()
        }
    }
    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(ZEN_DESTRUCTIVE)),
            text_color: Color::WHITE,
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: ZEN_DESTRUCTIVE,
            },
            ..Default::default()
        }
    }
    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
}
