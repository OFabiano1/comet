// Papéis de cor do Prism (prism/tokens/colors.css), tema escuro. Só este arquivo declara cor.

use ratatui::style::Color;

/// Void: fundo da tela.
pub const BACKGROUND: Color = Color::Rgb(0x09, 0x09, 0x0D);
/// Obsidian: superfície do campo.
pub const SURFACE: Color = Color::Rgb(0x10, 0x10, 0x15);

pub const TEXT_PRIMARY: Color = Color::Rgb(0xF0, 0xF0, 0xF4);
pub const TEXT_SECONDARY: Color = Color::Rgb(0x90, 0x90, 0x9D);
pub const TEXT_MUTED: Color = Color::Rgb(0x66, 0x66, 0x74);

// Escada do roxo: cada degrau tem um papel, o papel decide o token.
/// Lavender (`--text-brand`): todo texto violeta.
pub const TEXT_BRAND: Color = Color::Rgb(0xC0, 0x85, 0xFF);
/// Electric (`--prism-primary-hover`): barra de progresso.
pub const PRIMARY_HOVER: Color = Color::Rgb(0xA3, 0x4A, 0xFF);
/// Axolotl Purple (`--prism-primary`): logo grande, borda do campo em foco, item marcado.
pub const PRIMARY: Color = Color::Rgb(0x8F, 0x00, 0xFF);

pub const SUCCESS: Color = Color::Rgb(0x22, 0xC5, 0x5E);
pub const WARNING: Color = Color::Rgb(0xF5, 0x9E, 0x0B);
pub const DANGER: Color = Color::Rgb(0xEF, 0x44, 0x44);

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color) -> f64 {
        let Color::Rgb(r, g, b) = c else {
            panic!("só Rgb")
        };
        let ch = |v: u8| {
            let s = v as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
    }

    fn contrast(a: Color, b: Color) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Mesmos mínimos do Prism (scripts/check-tokens.test.mjs): texto 4,5:1, gráfico 3:1.
    #[test]
    fn prism_contrast_minimums_hold() {
        for bg in [BACKGROUND, SURFACE] {
            for text in [
                TEXT_PRIMARY,
                TEXT_SECONDARY,
                TEXT_BRAND,
                SUCCESS,
                WARNING,
                DANGER,
            ] {
                assert!(contrast(text, bg) >= 4.5, "{text:?} em {bg:?}");
            }
            for graphic in [PRIMARY, PRIMARY_HOVER] {
                assert!(contrast(graphic, bg) >= 3.0, "{graphic:?} em {bg:?}");
            }
        }
    }
}
