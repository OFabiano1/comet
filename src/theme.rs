// Papéis de cor do Prism (tokens/colors.css), tema escuro. Só este arquivo declara cor.

use ratatui::style::Color;

pub const BACKGROUND: Color = Color::Rgb(0x09, 0x09, 0x0D);
pub const TEXT_PRIMARY: Color = Color::Rgb(0xF0, 0xF0, 0xF4);
pub const TEXT_SECONDARY: Color = Color::Rgb(0x90, 0x90, 0x9D);
pub const TEXT_MUTED: Color = Color::Rgb(0x66, 0x66, 0x74);

/// Marca: logo e marcador de seleção.
pub const PRIMARY: Color = Color::Rgb(0x8F, 0x00, 0xFF);
/// Texto e borda em violeta: #8f00ff sobre o fundo fica em 3.4:1, o hover passa de 4.5:1.
pub const PRIMARY_HOVER: Color = Color::Rgb(0xA3, 0x4A, 0xFF);

pub const SUCCESS: Color = Color::Rgb(0x22, 0xC5, 0x5E);
pub const WARNING: Color = Color::Rgb(0xF5, 0x9E, 0x0B);
pub const DANGER: Color = Color::Rgb(0xEF, 0x44, 0x44);
