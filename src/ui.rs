use crate::app::{App, Item, Kind, Phase};
use crate::theme::{
    BACKGROUND, DANGER, PRIMARY, PRIMARY_HOVER, SUCCESS, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
    WARNING,
};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub const CONTENT_MAX_WIDTH: u16 = 64;
/// Mesmo convite do site (AxolotlBR/src/data/site.ts).
const DISCORD_INVITE: &str = "discord.gg/AxolotlBR";

pub fn truncate_ellipsis(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let keep = max - 1;
    s.chars().take(keep).collect::<String>() + "…"
}

/// Mantém o **final** do texto: com URL longa, o cursor continua visível.
pub fn tail_ellipsis(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let mut out = String::from("…");
    out.extend(s.chars().skip(count - (max - 1)));
    out
}

pub fn content_width(area_width: u16) -> u16 {
    area_width
        .saturating_sub(4)
        .clamp(1, CONTENT_MAX_WIDTH)
        .min(area_width)
}

pub fn centered_rect(area: Rect, height: u16) -> Rect {
    let width = content_width(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(width) / 2),
        y: area.y + (area.height.saturating_sub(h) / 2),
        width,
        height: h,
    }
}

fn fg(color: Color) -> Style {
    Style::new().fg(color)
}

/// Estado nunca só por cor: todo chip leva texto, e os finais levam ✓ ! ✗.
fn phase_chip(item: &Item) -> (String, Style) {
    match (item.phase, &item.error) {
        (Phase::Queued, _) => ("na fila".to_string(), fg(TEXT_MUTED)),
        (Phase::Downloading, _) => ("baixando".to_string(), fg(PRIMARY_HOVER)),
        (Phase::Converting, _) => ("convertendo pra gif".to_string(), fg(PRIMARY_HOVER)),
        (Phase::Done, None) => ("✓ pronto".to_string(), fg(SUCCESS)),
        (Phase::Done, Some(note)) => (format!("! {note}"), fg(WARNING)),
        (Phase::Failed, err) => (
            format!("✗ {}", err.as_deref().unwrap_or("o download falhou")),
            fg(DANGER),
        ),
    }
}

fn ext_of(item: &Item) -> &'static str {
    match item.kind {
        Kind::Video => "mp4",
        Kind::Gif => "gif",
    }
}

/// 64% · 12.4 MB / 19.3 MB — linha à direita do título durante o download.
fn progress_text(item: &Item) -> String {
    format!(
        "{:>3.0}% · {} / {}",
        item.pct,
        if item.got_str.is_empty() {
            "0 B".to_string()
        } else {
            item.got_str.clone()
        },
        if item.size_str.is_empty() {
            "…".to_string()
        } else {
            item.size_str.clone()
        }
    )
}

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    f.buffer_mut().set_style(area, Style::new().bg(BACKGROUND));

    let compact = area.height < 16;
    let spacer = if compact { 0 } else { 1 };
    let item_h = 3u16;
    let empty_h = 2u16;

    // Altura fixa: respiro + cabeçalho(4) + rótulo + input(3) + dica + fila(2) + rodapé.
    let header_h = 4u16;
    let fixed = spacer + header_h + spacer + 1 + 3 + 1 + spacer + 2 + spacer + 1;
    let space_for_body = area.height.saturating_sub(fixed);
    let visible = if app.items.is_empty() {
        0
    } else {
        let max_items = (space_for_body / item_h).max(1);
        (app.items.len() as u16).min(max_items)
    };
    let body_h = if app.items.is_empty() {
        empty_h
    } else {
        visible * item_h
    };

    let block = centered_rect(area, fixed + body_h);
    let chunks = ratatui::layout::Layout::vertical([
        ratatui::layout::Constraint::Length(spacer),
        ratatui::layout::Constraint::Length(header_h),
        ratatui::layout::Constraint::Length(spacer),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Length(3),
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Length(spacer),
        ratatui::layout::Constraint::Length(2),
        ratatui::layout::Constraint::Min(body_h),
        ratatui::layout::Constraint::Length(spacer),
        ratatui::layout::Constraint::Length(1),
    ])
    .split(block);

    // Logo, convite do Discord, versão e status — centralizados.
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "☄ COMET",
                fg(PRIMARY).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(DISCORD_INVITE, fg(TEXT_SECONDARY))),
            Line::from(Span::styled(
                format!("baixador de vídeo · v{}", env!("CARGO_PKG_VERSION")),
                fg(TEXT_MUTED),
            )),
            Line::from(Span::styled(app.status.clone(), fg(TEXT_SECONDARY))),
        ])
        .alignment(Alignment::Center),
        chunks[1],
    );

    // Rótulo do campo (overline).
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("LINK", fg(TEXT_SECONDARY)))),
        chunks[3],
    );

    // O campo está sempre em foco: borda no anel de foco do Prism.
    let input_rect = chunks[4];
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(fg(PRIMARY_HOVER));
    let inner = block.inner(input_rect);
    f.render_widget(block, input_rect);
    // 2 colunas reservadas: seta `↵` e cursor no fim do texto.
    let text_width = inner.width.saturating_sub(2) as usize;
    let (text, style) = if app.input.is_empty() {
        ("cola o link aqui (ctrl+v)".to_string(), fg(TEXT_MUTED))
    } else {
        (app.input.clone(), fg(TEXT_PRIMARY))
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            tail_ellipsis(&text, text_width),
            style,
        ))),
        Rect {
            width: inner.width.saturating_sub(2),
            ..inner
        },
    );
    if inner.width > 1 {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled("↵", fg(TEXT_MUTED))))
                .alignment(Alignment::Center),
            Rect {
                x: inner.x + inner.width - 1,
                width: 1,
                ..inner
            },
        );
    }
    // Cursor fica dentro do campo, depois do texto — limitado à área visível.
    if inner.height > 0 && inner.width > 0 {
        let cursor_x = inner.x + (app.input.chars().count() as u16).min(text_width as u16);
        f.set_cursor_position((
            cursor_x.min(area.width.saturating_sub(1)),
            inner.y.min(area.height.saturating_sub(1)),
        ));
    }

    // Dica de plataformas ou detecção da URL.
    let hint = match crate::sites::detect(&app.input) {
        Some(site) => Span::styled(format!("✓ link do {}", site.label()), fg(SUCCESS)),
        None if app.input.trim().is_empty() => {
            Span::styled("youtube · twitter/x · tiktok", fg(TEXT_MUTED))
        }
        None => Span::styled("✗ link não reconhecido", fg(DANGER)),
    };
    f.render_widget(Paragraph::new(Line::from(hint)), chunks[5]);

    // Cabeçalho da fila + contador.
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("FILA", fg(TEXT_SECONDARY))),
            Line::from(Span::styled(app.summary(), fg(TEXT_MUTED))),
        ]),
        chunks[7],
    );

    // Corpo da fila: itens ou estado vazio.
    let body = chunks[8];
    if app.items.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled("fila vazia", fg(TEXT_SECONDARY))),
                Line::from(Span::styled("cola um link e aperta enter", fg(TEXT_MUTED))),
            ])
            .alignment(Alignment::Center),
            body,
        );
    } else {
        let sel = app.selected.min(app.items.len().saturating_sub(1));
        let offset = sel.saturating_sub(visible.saturating_sub(1) as usize);
        let mut lines: Vec<Line> = Vec::new();
        for (i, item) in app
            .items
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible as usize)
        {
            let marker = if i == sel { "▎" } else { " " };
            let title_style = fg(if i == sel {
                TEXT_PRIMARY
            } else {
                TEXT_SECONDARY
            });
            let (chip, chip_style) = phase_chip(item);
            let meta = format!("{} · {}", item.site.label(), ext_of(item));
            let right = if item.phase == Phase::Downloading {
                progress_text(item)
            } else {
                String::new()
            };
            let avail = (body.width as usize).saturating_sub(1);
            let right_len = right.chars().count();
            let title_max = avail.saturating_sub(right_len + 1);
            let title = truncate_ellipsis(&item.title, title_max);
            // Progresso alinhado à direita da linha do título.
            let gap = avail.saturating_sub(title.chars().count() + right_len);
            lines.push(Line::from(vec![
                Span::styled(marker, fg(PRIMARY)),
                Span::styled(title, title_style),
                Span::styled(" ".repeat(gap), Style::default()),
                Span::styled(right, fg(PRIMARY_HOVER)),
            ]));
            lines.push(Line::from(Span::styled(format!(" {meta}"), fg(TEXT_MUTED))));
            lines.push(Line::from(Span::styled(format!(" {chip}"), chip_style)));
        }
        f.render_widget(Paragraph::new(lines), body);
    }

    // Rodapé compacto.
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "enter adiciona · d baixa · g gif · o pasta · q sai",
            fg(TEXT_MUTED),
        )))
        .alignment(Alignment::Center),
        chunks[10],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Kind, Phase};
    use crate::sites::Site;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render(app: &App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    fn render_grid(app: &App, w: u16, h: u16) -> Vec<Vec<String>> {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<Vec<String>>()
            })
            .collect()
    }

    #[test]
    fn truncate_handles_edge_cases() {
        assert_eq!(truncate_ellipsis("oi", 5), "oi");
        assert_eq!(truncate_ellipsis("hello world", 5), "hell…");
        assert_eq!(truncate_ellipsis("hello", 5), "hello");
        assert_eq!(truncate_ellipsis("hello", 0), "");
    }

    #[test]
    fn tail_keeps_end_of_long_text() {
        assert_eq!(tail_ellipsis("oi", 5), "oi");
        assert_eq!(tail_ellipsis("hello world", 5), "…orld");
        assert_eq!(tail_ellipsis("hello", 5), "hello");
        assert_eq!(tail_ellipsis("hello", 0), "");
        assert_eq!(tail_ellipsis("abcdef", 3).chars().count(), 3);
    }

    #[test]
    fn content_width_is_capped_and_never_overflows() {
        assert_eq!(content_width(80), 64);
        assert_eq!(content_width(68), 64);
        assert_eq!(content_width(40), 36);
        assert_eq!(content_width(20), 16);
        assert_eq!(content_width(4), 1);
        for w in [1u16, 5, 20, 40, 80, 200] {
            assert!(content_width(w) <= w, "overflow em {}", w);
        }
    }

    #[test]
    fn centered_rect_centers_block() {
        let area = Rect::new(0, 0, 80, 24);
        let r = centered_rect(area, 10);
        assert_eq!(r.x, 8);
        assert_eq!(r.y, 7);
        assert_eq!(r.width, 64);
        assert_eq!(r.height, 10);
    }

    #[test]
    fn centered_rect_clamps_when_taller_than_area() {
        let area = Rect::new(0, 0, 30, 6);
        let r = centered_rect(area, 20);
        assert_eq!(r.y, 0);
        assert_eq!(r.height, 6);
        assert!(r.width <= 30);
    }

    #[test]
    fn empty_state_shows_hint_and_footer() {
        let app = App::default();
        let text = render(&app, 80, 24);
        assert!(text.contains("☄ COMET"));
        assert!(text.contains("discord.gg/AxolotlBR"));
        assert!(text.contains("baixador de vídeo · v"));
        assert!(text.contains("cola o link aqui (ctrl+v)"));
        assert!(text.contains("youtube · twitter/x · tiktok"));
        assert!(text.contains("FILA"));
        assert!(text.contains("0 itens"));
        assert!(text.contains("fila vazia"));
        assert!(text.contains("cola um link e aperta enter"));
        assert!(text.contains("o pasta"));
    }

    fn item(phase: Phase, error: Option<&str>) -> Item {
        Item {
            url: "https://x.com/u/status/1".to_string(),
            title: "video title goes here".to_string(),
            site: Site::Twitter,
            kind: Kind::Gif,
            phase,
            pct: 64.0,
            size_str: "19.3 MB".to_string(),
            got_str: "12.4 MB".to_string(),
            error: error.map(str::to_string),
        }
    }

    #[test]
    fn final_states_carry_text_not_only_color() {
        for (phase, error, expected) in [
            (Phase::Done, None, "✓ pronto"),
            (
                Phase::Done,
                Some("gif falhou, o mp4 ficou salvo"),
                "! gif falhou, o mp4 ficou salvo",
            ),
            (
                Phase::Failed,
                Some("link não suportado"),
                "✗ link não suportado",
            ),
            (Phase::Failed, None, "✗ o download falhou"),
        ] {
            let app = App {
                items: vec![item(phase, error)],
                ..App::default()
            };
            assert!(render(&app, 80, 24).contains(expected), "faltou {expected}");
        }
    }

    #[test]
    fn discord_invite_sits_right_under_the_logo() {
        let grid = render_grid(&App::default(), 80, 24);
        let row_of = |needle: &str| {
            grid.iter()
                .position(|row| row.concat().contains(needle))
                .unwrap()
        };
        assert_eq!(row_of("discord.gg/AxolotlBR"), row_of("☄") + 1);
    }

    #[test]
    fn layout_is_centered_not_left_anchored() {
        let app = App::default();
        let grid = render_grid(&app, 80, 24);
        // nenhuma coluna antes do conteúdo (logo começa após a margem)
        let logo_col = grid
            .iter()
            .find_map(|row| row.iter().position(|c| c.contains("☄")))
            .unwrap();
        assert!(logo_col >= 8, "logo em coluna {}", logo_col);
        // margem esquerda vazia em todas as linhas
        for row in &grid {
            for cell in row.iter().take(8) {
                assert_eq!(cell, " ", "conteudo colado na margem esquerda");
            }
        }
    }

    #[test]
    fn queue_item_shows_title_and_progress() {
        let app = App {
            input: "https://x.com/u/status/1".to_string(),
            items: vec![item(Phase::Downloading, None)],
            ..App::default()
        };
        let text = render(&app, 80, 24);
        assert!(text.contains("video title goes here"));
        assert!(text.contains("64%"));
        assert!(text.contains("✓ link do twitter"));
    }

    #[test]
    fn tiny_terminal_does_not_panic() {
        let app = App {
            input: "https://x.com/u/status/1".to_string(),
            ..App::default()
        };
        render(&app, 20, 8);
        render(&app, 10, 4);
    }
}
