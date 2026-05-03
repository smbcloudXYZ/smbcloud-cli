//! Shared smbCloud brand colour palette for all TUI views.
//!
//! Sourced 1-to-1 from `smbcloud-web/tailwind.config.ts` `colors.brand` scale.
//!
//!  brand-600  #E11D48  → primary accent (titles, column headers, key hints)
//!  brand-700  #BE123C  → active borders / scrollbar thumb
//!  brand-800  #9F1239  → selected-row / key-hint pill background
//!  brand-300  #FDA4AF  → selected-row foreground
use ratatui::style::Color;

pub const BRAND: Color = Color::Rgb(225, 29, 72); //  #E11D48  brand-600
pub const BRAND_MID: Color = Color::Rgb(190, 18, 60); //  #BE123C  brand-700
pub const BRAND_DARK: Color = Color::Rgb(159, 18, 57); //  #9F1239  brand-800
pub const BRAND_LIGHT: Color = Color::Rgb(253, 164, 175); //  #FDA4AF  brand-300

pub const BG: Color = Color::Rgb(9, 9, 11); //  near-black canvas
pub const BG_SURFACE: Color = Color::Rgb(18, 18, 24); //  elevated chrome (bars)
pub const BG_HEADER_ROW: Color = Color::Rgb(26, 26, 36); //  column-header band

pub const TEXT_PRIMARY: Color = Color::Rgb(241, 241, 243); //  off-white body
pub const TEXT_MUTED: Color = Color::Rgb(113, 113, 122); //  zinc-500 secondary

pub const BORDER_IDLE: Color = Color::Rgb(39, 39, 42); //  zinc-800 resting
pub const BORDER_ACTIVE: Color = Color::Rgb(63, 63, 70); //  zinc-700 focused
