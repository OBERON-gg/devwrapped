use crate::draw::Font;
use crate::fonts;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Night,
    Poster,
    Paper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    Afterglow,
    Zine,
    Swiss,
}

pub(crate) struct Palette {
    pub background: &'static str,
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub second: &'static str,
    /// the colors of the glowing blobs behind afterglow slides
    pub glows: [&'static str; 3],
}

pub(crate) const MONO: Font = Font {
    family: fonts::MONO_FAMILY,
    weight: 400,
};

pub(crate) const MONO_BOLD: Font = Font {
    family: fonts::MONO_FAMILY,
    weight: 700,
};

impl Theme {
    pub(crate) fn layout(self) -> Layout {
        match self {
            Theme::Night => Layout::Afterglow,
            Theme::Poster => Layout::Swiss,
            Theme::Paper => Layout::Zine,
        }
    }

    pub(crate) fn display(self, weight: u16) -> Font {
        let family = match self.layout() {
            Layout::Afterglow => fonts::UNBOUNDED_FAMILY,
            Layout::Zine | Layout::Swiss => fonts::SPACE_GROTESK_FAMILY,
        };
        Font { family, weight }
    }

    pub(crate) fn display_font(self) -> &'static [u8] {
        match self.layout() {
            Layout::Afterglow => fonts::UNBOUNDED,
            Layout::Zine | Layout::Swiss => fonts::SPACE_GROTESK,
        }
    }

    pub(crate) fn palette(self) -> Palette {
        match self {
            Theme::Night => Palette {
                background: "#0c0a1d",
                text: "#ffffff",
                muted: "#a9a3d6",
                accent: "#c8ff3d",
                second: "#22d3ee",
                glows: ["#ff3d9a", "#6a4bff", "#22d3ee"],
            },
            Theme::Paper => Palette {
                background: "#f3ede1",
                text: "#1d1b18",
                muted: "#6f685d",
                accent: "#e0402b",
                second: "#2643a6",
                glows: ["#f3ede1"; 3],
            },
            Theme::Poster => Palette {
                background: "#ffffff",
                text: "#111111",
                muted: "#555555",
                accent: "#e3120b",
                second: "#111111",
                glows: ["#ffffff"; 3],
            },
        }
    }
}
