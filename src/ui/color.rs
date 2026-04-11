#![allow(unused)]
use egui::Color32;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

pub struct Colors;

impl Colors {
    pub const BACKDROP:  Color32 = Color32::from_rgb(18,   8,  42);  // #12082A — deep indigo-black
    pub const BASE:      Color32 = Color32::from_rgb(42,  22,  80);  // #2A1650 — dark purple surface
    pub const HIGHLIGHT: Color32 = Color32::from_rgb(61,  34, 112);  // #3D2270 — selected/hover state
    pub const SUBTEXT:   Color32 = Color32::from_rgb(107, 84, 144);  // #6B5490 — muted purple text
    pub const TEXT: Color32 = Color32::from_rgb(190, 170, 155);
    pub const RED: Color32 = Color32::from_rgb(240, 100, 100);
    pub const ORANGE: Color32 = Color32::from_rgb(240, 140, 90);
    pub const YELLOW: Color32 = Color32::from_rgb(240, 200, 120);
    pub const GREEN: Color32 = Color32::from_rgb(160, 240, 130);
    pub const TEAL: Color32 = Color32::from_rgb(80, 200, 200);
    pub const BLUE: Color32 = Color32::from_rgb(100, 150, 240);
    pub const PURPLE: Color32 = Color32::from_rgb(180, 120, 240);

    pub const ACCENT_COLORS: [(&str, Color32); 7] = [
        ("Red", Self::RED),
        ("Orange", Self::ORANGE),
        ("Yellow", Self::YELLOW),
        ("Green", Self::GREEN),
        ("Teal", Self::TEAL),
        ("Blue", Self::BLUE),
        ("Purple", Self::PURPLE),
    ];
}

/// Menu accent style — selects an entire color palette for the UI.
#[derive(Debug, Clone, Copy, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum AccentStyle {
    /// Current purple scheme (default)
    Default,
    /// Deep navy + cyan with orange complement (complementary colors)
    Ocean,
    /// Warm coral/salmon with brown tones (analogous warm colors)
    Sunrise,
    /// Lime green on near-black for maximum contrast (matrix aesthetic)
    Neon,
    /// Hot pink with gold complement (triadic color theory)
    Retro,
    /// Electric blue on professional cool grays
    Speclist,
    /// Classic red and gold fast-food branding
    McDonalds,
    /// Soft muted pastel tones
    Pastel,
    /// Mint green accent with soft pink complement
    GreenPink,
    /// Soft butter yellow accent with lavender purple complement
    YellowPurple,
    //White
    Winter,

    /// Gentle peachy coral tones
    SoftCoral,
    Fanta,
    
    /// Night mode themes
    /// Deep dark blue with minimal contrast for comfortable night viewing
    NightBlue,
    /// Warm amber tones that reduce blue light significantly
    NightAmber,
    /// Pure grayscale theme for maximum eye comfort
    NightGray,
    /// Dark green theme inspired by terminal emulators
    NightTerminal,
    /// Skeet-style theme (cyan→purple→yellow accent, dark panels)
    Skeet,
}

/// A full color palette derived from an [`AccentStyle`].
#[derive(Debug, Clone)]
pub struct ColorScheme {
    /// Deepest background (window fill, extreme bg)
    pub backdrop: Color32,
    /// Main UI surface (panels, cards)
    pub base: Color32,
    /// Hover / selected state
    pub highlight: Color32,
    /// Muted text / border color
    pub subtext: Color32,
    /// Primary accent color (selection, active widgets)
    pub accent: Color32,
    /// Brighter accent variant
    pub accent_bright: Color32,
    /// Complement / warning color
    pub complement: Color32,
}

impl ColorScheme {
    pub fn for_style(style: &AccentStyle) -> Self {
        match style {
            AccentStyle::Default => Self {
                backdrop:      Color32::from_rgb(18,   8,  42),
                base:          Color32::from_rgb(42,  22,  80),
                highlight:     Color32::from_rgb(61,  34, 112),
                subtext:       Color32::from_rgb(107, 84, 144),
                accent:        Color32::from_rgb(180, 120, 240),
                accent_bright: Color32::from_rgb(200, 150, 255),
                complement:    Color32::from_rgb(255, 150, 100),
            },
            AccentStyle::Ocean => Self {
                backdrop:      Color32::from_rgb(10,  20,  45),
                base:          Color32::from_rgb(15,  29,  61),
                highlight:     Color32::from_rgb(26,  58,  82),
                subtext:       Color32::from_rgb(80,  140, 180),
                accent:        Color32::from_rgb(0,  217, 255),
                accent_bright: Color32::from_rgb(100, 230, 255),
                complement:    Color32::from_rgb(255, 107,  53),
            },
            AccentStyle::Sunrise => Self {
                backdrop:      Color32::from_rgb(30,  14,   8),
                base:          Color32::from_rgb(42,  24,  16),
                highlight:     Color32::from_rgb(74,  44,  26),
                subtext:       Color32::from_rgb(180, 130, 100),
                accent:        Color32::from_rgb(255, 107,  91),
                accent_bright: Color32::from_rgb(255, 138, 117),
                complement:    Color32::from_rgb(255, 200,  80),
            },
            AccentStyle::Neon => Self {
                backdrop:      Color32::from_rgb(5,    8,  15),
                base:          Color32::from_rgb(10,  14,  39),
                highlight:     Color32::from_rgb(15,  50,  20),
                subtext:       Color32::from_rgb(80,  160, 100),
                accent:        Color32::from_rgb(0,  255,  65),
                accent_bright: Color32::from_rgb(0,  255, 127),
                complement:    Color32::from_rgb(255, 255,   0),
            },
            AccentStyle::Retro => Self {
                backdrop:      Color32::from_rgb(15,   0,  30),
                base:          Color32::from_rgb(26,   0,  51),
                highlight:     Color32::from_rgb(51,   0, 102),
                subtext:       Color32::from_rgb(180, 100, 200),
                accent:        Color32::from_rgb(255,   0, 110),
                accent_bright: Color32::from_rgb(255,  80, 180),
                complement:    Color32::from_rgb(255, 183,   3),
            },
            AccentStyle::Speclist => Self {
                backdrop:      Color32::from_rgb(17,  24,  39),
                base:          Color32::from_rgb(31,  41,  55),
                highlight:     Color32::from_rgb(55,  65,  81),
                subtext:       Color32::from_rgb(156, 163, 175),
                accent:        Color32::from_rgb(0,  128, 255),
                accent_bright: Color32::from_rgb(59,  130, 246),
                complement:    Color32::from_rgb(16,  185, 129),
            },
            AccentStyle::McDonalds => Self {
                backdrop:      Color32::from_rgb(30,   8,   5),
                base:          Color32::from_rgb(50,  15,  10),
                highlight:     Color32::from_rgb(75,  22,  15),
                subtext:       Color32::from_rgb(205, 145, 125),
                accent:        Color32::from_rgb(255, 200,  40),
                accent_bright: Color32::from_rgb(255, 220,  90),
                complement:    Color32::from_rgb(220,  45,  28),
            },
            AccentStyle::Pastel => Self {
                backdrop:      Color32::from_rgb(15,  28,  30),
                base:          Color32::from_rgb(25,  48,  50),
                highlight:     Color32::from_rgb(38,  72,  74),
                subtext:       Color32::from_rgb(148, 208, 200),
                accent:        Color32::from_rgb(160, 235, 215),
                accent_bright: Color32::from_rgb(190, 250, 235),
                complement:    Color32::from_rgb(250, 200, 180),
            },
            AccentStyle::GreenPink => Self {
                backdrop:      Color32::from_rgb(30,  15,  25),
                base:          Color32::from_rgb(48,  25,  38),
                highlight:     Color32::from_rgb(68,  38,  55),
                subtext:       Color32::from_rgb(200, 165, 180),
                accent:        Color32::from_rgb(160, 235, 130),
                accent_bright: Color32::from_rgb(185, 250, 155),
                complement:    Color32::from_rgb(240, 130, 175),
            },
            AccentStyle::YellowPurple => Self {
                backdrop:      Color32::from_rgb(18,  12,  35),   // deep purple-black
                base:          Color32::from_rgb(35,  25,  62),   // dark purple surface
                highlight:     Color32::from_rgb(62,  42, 105),   // purple hover
                subtext:       Color32::from_rgb(205, 190, 240),  // light lavender subtext
                accent:        Color32::from_rgb(255, 210,  45),  // vibrant yellow
                accent_bright: Color32::from_rgb(255, 232,  90),  // bright yellow
                complement:    Color32::from_rgb(185, 135, 255),  // vivid purple complement
            },
             AccentStyle::Winter => Self {
                backdrop:      Color32::from_rgb(245, 245, 245),  // very light gray (#F5F5F5)
                base:          Color32::from_rgb(255, 255, 255),  // pure white (#FFFFFF)
                highlight:     Color32::from_rgb(232, 232, 232),  // light gray hover (#E8E8E8)
                subtext:       Color32::from_rgb(102, 102, 102),  // medium gray text (#666666)
                accent:        Color32::from_rgb(74,  144, 226),  // cool blue (#4A90E2)
                accent_bright: Color32::from_rgb(91,  163, 245),  // light blue (#5BA3F5)
                complement:    Color32::from_rgb(30,  144, 255),  // dodger blue (#1E90FF)
            },
            AccentStyle::SoftCoral => Self {
                backdrop:      Color32::from_rgb(35,  18,  18),
                base:          Color32::from_rgb(55,  30,  28),
                highlight:     Color32::from_rgb(80,  48,  44),
                subtext:       Color32::from_rgb(210, 165, 155),
                accent:        Color32::from_rgb(240, 170, 155),
                accent_bright: Color32::from_rgb(255, 195, 180),
                complement:    Color32::from_rgb(255, 215, 160),
            },
            AccentStyle::Fanta => Self {
                backdrop:      Color32::from_rgb(28,  18,   2),  // dark yellow-brown
                base:          Color32::from_rgb(55,  38,   5),  // deep amber-brown
                highlight:     Color32::from_rgb(90,  65,  10),  // warm yellow-brown hover
                subtext:       Color32::from_rgb(210, 175,  80),  // muted yellow-amber text
                accent:        Color32::from_rgb(255, 185,   0),  // #FFB900 — warm amber-yellow
                accent_bright: Color32::from_rgb(255, 240, 100),  // #FFF064 — bright lemon yellow
                complement:    Color32::from_rgb(255, 210, 140),  // warm light peach
            },
            AccentStyle::NightBlue => Self {
                backdrop:      Color32::from_rgb(8,   12,  20),   // very dark blue
                base:          Color32::from_rgb(12,  18,  30),   // dark blue surface
                highlight:     Color32::from_rgb(18,  26,  42),   // subtle blue highlight
                subtext:       Color32::from_rgb(120, 140, 160),  // muted blue-gray text
                accent:        Color32::from_rgb(100, 150, 200),  // soft blue accent
                accent_bright: Color32::from_rgb(130, 180, 230),  // brighter blue accent
                complement:    Color32::from_rgb(180, 200, 220),  // light blue complement
            },
            AccentStyle::NightAmber => Self {
                backdrop:      Color32::from_rgb(20,  15,   8),   // dark amber
                base:          Color32::from_rgb(30,  22,  12),   // warm dark surface
                highlight:     Color32::from_rgb(45,  34,  20),   // warm amber highlight
                subtext:       Color32::from_rgb(180, 160, 120),  // warm amber text
                accent:        Color32::from_rgb(220, 180, 100),  // warm amber accent
                accent_bright: Color32::from_rgb(240, 200, 120),  // bright amber accent
                complement:    Color32::from_rgb(200, 150,  80),  // muted amber complement
            },
            AccentStyle::NightGray => Self {
                backdrop:      Color32::from_rgb(15,  15,  15),   // near-black
                base:          Color32::from_rgb(25,  25,  25),   // dark gray
                highlight:     Color32::from_rgb(35,  35,  35),   // medium gray
                subtext:       Color32::from_rgb(140, 140, 140),  // medium gray text
                accent:        Color32::from_rgb(180, 180, 180),  // light gray accent
                accent_bright: Color32::from_rgb(210, 210, 210),  // bright gray accent
                complement:    Color32::from_rgb(160, 160, 160),  // medium-light gray
            },
            AccentStyle::NightTerminal => Self {
                backdrop:      Color32::from_rgb(10,  20,  10),   // dark green terminal
                base:          Color32::from_rgb(15,  30,  15),   // terminal green surface
                highlight:     Color32::from_rgb(20,  40,  20),   // green highlight
                subtext:       Color32::from_rgb(120, 180, 120),  // terminal green text
                accent:        Color32::from_rgb(100, 255, 100),  // bright green accent
                accent_bright: Color32::from_rgb(150, 255, 150),  // very bright green
                complement:    Color32::from_rgb(200, 200, 100),  // yellow-green complement
            },
            AccentStyle::Skeet => Self {
                backdrop:      Color32::from_rgb(15,  15,  15),   // near-black panel fill
                base:          Color32::from_rgb(30,  30,  30),   // dark gray surface
                highlight:     Color32::from_rgb(45,  45,  45),   // hover state
                subtext:       Color32::from_rgb(160, 160, 160),  // muted gray text
                accent:        Color32::from_rgb(0,  213, 255),   // skeet cyan
                accent_bright: Color32::from_rgb(204, 18, 204),   // skeet magenta
                complement:    Color32::from_rgb(255, 250,   0),  // skeet yellow
            },
        }
    }
}
