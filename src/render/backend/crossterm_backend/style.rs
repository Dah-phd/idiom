use crossterm::style::{Attribute, Attributes, ContentStyle};

use super::Color;

#[derive(Debug, Default, Clone, Copy)]
pub struct Style(ContentStyle);

impl Style {
    #[inline]
    pub fn update(&mut self, rhs: Self) {
        if let Some(c) = rhs.0.foreground_color {
            self.0.foreground_color.replace(c);
        }
        if let Some(c) = rhs.0.background_color {
            self.0.background_color.replace(c);
        }
        if let Some(c) = rhs.0.underline_color {
            self.0.underline_color.replace(c);
        }
        self.0.attributes = rhs.0.attributes;
    }

    #[inline]
    pub fn set_fg(&mut self, color: Option<Color>) {
        self.0.foreground_color = color;
    }

    #[inline]
    pub fn fg(color: Color) -> Self {
        Self(ContentStyle {
            foreground_color: Some(color),
            background_color: None,
            underline_color: None,
            attributes: Attributes::default(),
        })
    }

    #[inline]
    pub fn set_bg(&mut self, color: Option<Color>) {
        self.0.background_color = color;
    }

    #[inline]
    pub fn bg(color: Color) -> Self {
        Self(ContentStyle {
            background_color: Some(color),
            foreground_color: None,
            underline_color: None,
            attributes: Attributes::default(),
        })
    }

    #[inline]
    pub fn drop_bg(&mut self) {
        self.0.background_color = None;
    }

    #[inline]
    pub fn add_slowblink(&mut self) {
        self.0.attributes.set(Attribute::SlowBlink);
    }

    #[inline]
    pub fn slowblink() -> Self {
        Self(ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: None,
            attributes: Attribute::SlowBlink.into(),
        })
    }

    #[inline]
    pub fn add_bold(&mut self) {
        self.0.attributes.set(Attribute::Bold);
    }

    #[inline]
    pub fn bold() -> Self {
        Self(ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: None,
            attributes: Attribute::Bold.into(),
        })
    }

    #[inline]
    pub fn add_reverse(&mut self) {
        self.0.attributes.set(Attribute::Reverse);
    }

    #[inline]
    pub fn reversed() -> Self {
        Self(ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: None,
            attributes: Attribute::Reverse.into(),
        })
    }

    #[inline]
    pub fn reset_mods(&mut self) {
        self.0.attributes = Attributes::default();
        self.0.underline_color = None;
    }

    #[inline]
    pub fn undercurle(&mut self, color: Option<Color>) {
        self.0.attributes.set(Attribute::Undercurled);
        self.0.underline_color = color;
    }

    #[inline]
    pub fn undercurled(color: Option<Color>) -> Self {
        Self(ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: color,
            attributes: Attribute::Undercurled.into(),
        })
    }

    #[inline]
    pub fn underline(&mut self, color: Option<Color>) {
        self.0.attributes.set(Attribute::Underlined);
        self.0.underline_color = color;
    }

    #[inline]
    pub fn underlined(color: Option<Color>) -> Self {
        Self(ContentStyle {
            background_color: None,
            foreground_color: None,
            underline_color: color,
            attributes: Attribute::Underlined.into(),
        })
    }
}

impl Into<ContentStyle> for Style {
    #[inline]
    fn into(self) -> ContentStyle {
        self.0
    }
}

impl Into<ContentStyle> for &Style {
    #[inline]
    fn into(self) -> ContentStyle {
        self.0.clone()
    }
}
