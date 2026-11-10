use crate::app::{App, Item, Kind, Phase, Progress};
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

/// Logo do README. Todas as linhas com a mesma largura: o centro alinha o bloco, não cada linha.
const LOGO: [&str; 5] = [
    r"                          _        ",
    r"  ___ ___  _ __ ___   ___| |_  _   ",
    r" / __/ _ \| '_ ` _ \ / _ \ __|| |_ ",
    r"| (_| (_) | | | | | |  __/ ||_   _|",
    r" \___\___/|_| |_| |_|\___|\__||_|  ",
];
const LOGO_WIDTH: u16 = 35;
/// Abaixo disso o logo grande come a fila; volta o "☄ COMET" de uma linha.
const LOGO_MIN_HEIGHT: u16 = 28;

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
        (Phase::Done, None) => match item.saved.as_deref().and_then(short_folder) {
            Some(folder) => (format!("✓ salvo em {folder} · o mostra"), fg(SUCCESS)),
            None => ("✓ pronto".to_string(), fg(SUCCESS)),
        },
        (Phase::Done, Some(note)) => (format!("! {note}"), fg(WARNING)),
        (Phase::Failed, err) => (
            format!("✗ {}", err.as_deref().unwrap_or("o download falhou")),
            fg(DANGER),
        ),
    }
}

/// `C:\Users\x\Downloads\Comet\a.mp4` → `Downloads\Comet`: o bastante pra achar a pasta.
pub fn short_folder(file: &std::path::Path) -> Option<String> {
    let dir = file.parent()?;
    let name = dir.file_name()?.to_string_lossy();
    Some(match dir.parent().and_then(|p| p.file_name()) {
        Some(up) => format!("{}\\{name}", up.to_string_lossy()),
        None => name.to_string(),
    })
}

fn ext_of(item: &Item) -> &'static str {
    match item.kind {
        Kind::Video => "mp4",
        Kind::Gif => "gif",
    }
}

/// 850 KB · 6.9 MB · 1.2 GB (base 1000, como o explorador do Windows mostra pra leigo).
pub fn human_bytes(b: u64) -> String {
    let b = b as f64;
    if b < 1e6 {
        format!("{:.0} KB", (b / 1e3).max(1.0))
    } else if b < 1e9 {
        format!("{:.1} MB", b / 1e6)
    } else {
        format!("{:.1} GB", b / 1e9)
    }
}

/// ~12s · ~3min · ~1h05
pub fn human_eta(secs: u64) -> String {
    match secs {
        0..=59 => format!("~{secs}s"),
        60..=3599 => format!("~{}min", secs.div_ceil(60)),
        _ => format!("~{}h{:02}", secs / 3600, (secs % 3600) / 60),
    }
}

/// 0:19 · 3:45 · 1:02:03
pub fn human_duration(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// À direita do título: "faltam 6.9 MB · ~12s", ou só o baixado se o total não veio.
fn remaining_text(p: &Progress) -> String {
    let Some(left) = p.remaining() else {
        return format!("{} baixados", human_bytes(p.downloaded));
    };
    match p.eta_secs {
        Some(eta) => format!("faltam {} · {}", human_bytes(left), human_eta(eta)),
        None => format!("faltam {}", human_bytes(left)),
    }
}

/// "youtube · mp4 · jawed · 0:19" — canal e duração entram quando o yt-dlp manda.
fn meta_text(item: &Item) -> String {
    let mut parts = vec![item.site.label().to_string(), ext_of(item).to_string()];
    if !item.info.channel.is_empty() {
        parts.push(item.info.channel.clone());
    }
    if let Some(d) = item.info.duration_secs {
        parts.push(human_duration(d));
    }
    parts.join(" · ")
}

/// "baixando vídeo ███████░░░░ 64% · 2.1 MB/s", cabendo em `width` colunas.
fn progress_line(p: &Progress, width: usize) -> Line<'static> {
    let label = if p.audio {
        "baixando áudio "
    } else {
        "baixando vídeo "
    };
    let mut tail = match p.pct() {
        Some(pct) => format!(" {pct:.0}%"),
        None => " …".to_string(),
    };
    if let Some(speed) = p.speed {
        tail.push_str(&format!(" · {}/s", human_bytes(speed as u64)));
    }
    let bar_w = width.saturating_sub(label.chars().count() + tail.chars().count());
    // Barra curta demais não informa nada: some e deixa só o número.
    let (filled, empty) = if bar_w < 4 {
        (0, 0)
    } else {
        let done = (p.pct().unwrap_or(0.0) / 100.0 * bar_w as f64).round() as usize;
        (done.min(bar_w), bar_w - done.min(bar_w))
    };
    Line::from(vec![
        Span::styled(label, fg(PRIMARY_HOVER)),
        Span::styled("█".repeat(filled), fg(PRIMARY_HOVER)),
        Span::styled("░".repeat(empty), fg(TEXT_MUTED)),
        Span::styled(tail, fg(TEXT_SECONDARY)),
    ])
}

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    f.buffer_mut().set_style(area, Style::new().bg(BACKGROUND));

    let compact = area.height < 16;
    let spacer = if compact { 0 } else { 1 };
    let item_h = 3u16;
    let empty_h = 2u16;

    let big_logo = area.height >= LOGO_MIN_HEIGHT && content_width(area.width) >= LOGO_WIDTH;
    // Altura fixa: respiro + cabeçalho + rótulo + input(3) + dica + fila(2) + rodapé.
    let logo_h = if big_logo { LOGO.len() as u16 } else { 1 };
    let header_h = logo_h + 3;
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
    let logo_style = fg(PRIMARY).add_modifier(Modifier::BOLD);
    let mut header: Vec<Line> = if big_logo {
        LOGO.iter()
            .map(|l| Line::from(Span::styled(*l, logo_style)))
            .collect()
    } else {
        vec![Line::from(Span::styled("☄ COMET", logo_style))]
    };
    header.extend([
        Line::from(Span::styled(DISCORD_INVITE, fg(TEXT_SECONDARY))),
        Line::from(Span::styled(
            format!("baixador de vídeo · v{}", env!("CARGO_PKG_VERSION")),
            fg(TEXT_MUTED),
        )),
        Line::from(Span::styled(app.status.clone(), fg(TEXT_SECONDARY))),
    ]);
    f.render_widget(
        Paragraph::new(header).alignment(Alignment::Center),
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
            let downloading = item.phase == Phase::Downloading;
            let right = if downloading {
                remaining_text(&item.progress)
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
                Span::styled(right, fg(TEXT_SECONDARY)),
            ]));
            lines.push(Line::from(Span::styled(
                format!(" {}", meta_text(item)),
                fg(TEXT_MUTED),
            )));
            if downloading {
                let mut bar = progress_line(&item.progress, avail);
                bar.spans.insert(0, Span::raw(" "));
                lines.push(bar);
            } else {
                let (chip, chip_style) = phase_chip(item);
                lines.push(Line::from(Span::styled(format!(" {chip}"), chip_style)));
            }
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
    use crate::app::{App, Kind, Phase, VideoInfo};
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
            info: VideoInfo {
                channel: "jawed".to_string(),
                duration_secs: Some(19),
            },
            progress: Progress {
                downloaded: 12_400_000,
                total: Some(19_300_000),
                speed: Some(2_100_000.0),
                eta_secs: Some(3),
                audio: false,
            },
            error: error.map(str::to_string),
            saved: None,
        }
    }

    #[test]
    fn final_states_carry_text_not_only_color() {
        let mut saved = item(Phase::Done, None);
        saved.saved = Some(std::path::PathBuf::from(
            "C:\\Users\\F\\Downloads\\Comet\\Layne [1].mp4",
        ));
        let app = App {
            items: vec![saved],
            ..App::default()
        };
        assert!(render(&app, 80, 24).contains("✓ salvo em Downloads\\Comet · o mostra"));

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
    fn logo_lines_share_one_width() {
        for line in LOGO {
            assert_eq!(line.chars().count(), LOGO_WIDTH as usize, "{line:?}");
        }
    }

    #[test]
    fn tall_terminal_gets_ascii_logo_with_invite_below() {
        let grid = render_grid(&App::default(), 80, LOGO_MIN_HEIGHT);
        let row_of = |needle: &str| grid.iter().position(|r| r.concat().contains(needle));
        let last_logo_row = row_of(LOGO[4]).expect("logo grande");
        assert_eq!(row_of(LOGO[0].trim()), Some(last_logo_row - 4));
        assert_eq!(row_of("discord.gg/AxolotlBR"), Some(last_logo_row + 1));
        assert_eq!(row_of("☄"), None);
        // Um terminal baixo volta pro logo de uma linha.
        assert!(render(&App::default(), 80, LOGO_MIN_HEIGHT - 1).contains("☄ COMET"));
    }

    /// Cor do ratatui em CSS; Reset fica com o fundo do Prism.
    fn css(color: Color) -> String {
        match color {
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => "inherit".to_string(),
        }
    }

    /// Tela renderizada em HTML, uma célula por caractere (largura fixa, nada desalinha).
    fn screenshot_html(app: &App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let mut rows = String::new();
        for y in 0..h {
            rows.push_str("<div>");
            for x in 0..w {
                let cell = &buf[(x, y)];
                let sym = match cell.symbol() {
                    "<" => "&lt;",
                    ">" => "&gt;",
                    "&" => "&amp;",
                    s => s,
                };
                let bold = if cell.modifier.contains(Modifier::BOLD) {
                    ";font-weight:700"
                } else {
                    ""
                };
                rows.push_str(&format!(
                    "<i style=\"color:{};background:{}{bold}\">{sym}</i>",
                    css(cell.fg),
                    css(cell.bg)
                ));
            }
            rows.push_str("</div>\n");
        }
        format!(
            "<!doctype html><meta charset=\"utf-8\"><style>\
@font-face{{font-family:JBM;src:url(JetBrainsMono.woff2)}}\
body{{margin:0;background:{bg}}}\
.t{{padding:24px;font:15px/20px JBM,'Cascadia Mono',Consolas,monospace;color:#f0f0f4}}\
.t div{{height:20px;white-space:pre}}\
.t i{{font-style:normal;display:inline-block;width:10px;height:20px;text-align:center;overflow:visible}}\
</style><div class=\"t\">\n{rows}</div>",
            bg = css(BACKGROUND)
        )
    }

    /// Gera as telas do README: `.\screenshots.ps1` (roda isto e tira o PNG no Chromium).
    #[test]
    #[ignore = "gera arquivos; use .\\screenshots.ps1"]
    fn render_screenshots() {
        use std::path::PathBuf;
        let out = PathBuf::from(
            std::env::var("COMET_SHOT_DIR").unwrap_or_else(|_| "target/screenshots".into()),
        );
        std::fs::create_dir_all(&out).unwrap();

        let zoo = Item {
            url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".into(),
            title: "Me at the zoo".into(),
            site: Site::YouTube,
            kind: Kind::Video,
            phase: Phase::Downloading,
            info: VideoInfo {
                channel: "jawed".into(),
                duration_secs: Some(19),
            },
            progress: Progress {
                downloaded: 143_220,
                total: Some(223_779),
                speed: Some(120_000.0),
                eta_secs: Some(1),
                audio: false,
            },
            error: None,
            saved: None,
        };
        let tweet = Item {
            url: "https://x.com/i/status/1".into(),
            title: "Layne".into(),
            site: Site::Twitter,
            kind: Kind::Gif,
            phase: Phase::Queued,
            info: VideoInfo {
                channel: "Layne".into(),
                duration_secs: Some(10),
            },
            progress: Progress::default(),
            error: None,
            saved: None,
        };
        let downloads = PathBuf::from(r"C:\Users\voce\Downloads\Comet");
        let done = |mut it: Item, file: &str| {
            it.phase = Phase::Done;
            it.saved = Some(downloads.join(file));
            it
        };

        let telas = [
            ("1-inicio", App::default()),
            (
                "2-baixando",
                App {
                    status: crate::app::STATUS_BUSY.into(),
                    items: vec![zoo.clone(), tweet.clone()],
                    busy: true,
                    ..App::default()
                },
            ),
            (
                "3-pronto",
                App {
                    items: vec![done(zoo, "Me at the zoo.mp4"), done(tweet, "Layne [1].gif")],
                    completed: 2,
                    selected: 1,
                    ..App::default()
                },
            ),
        ];
        for (name, app) in telas {
            let html = screenshot_html(&app, 80, 32);
            std::fs::write(out.join(format!("{name}.html")), html).unwrap();
        }
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
        assert!(text.contains("faltam 6.9 MB · ~3s"));
        assert!(text.contains("twitter · gif · jawed · 0:19"));
        assert!(text.contains("baixando vídeo █"));
        assert!(text.contains("░ 64% · 2.1 MB/s"));
        assert!(text.contains("✓ link do twitter"));
    }

    #[test]
    fn human_formats() {
        assert_eq!(human_bytes(0), "1 KB");
        assert_eq!(human_bytes(850_000), "850 KB");
        assert_eq!(human_bytes(6_900_000), "6.9 MB");
        assert_eq!(human_bytes(1_200_000_000), "1.2 GB");
        assert_eq!(human_eta(12), "~12s");
        assert_eq!(human_eta(61), "~2min");
        assert_eq!(human_eta(3900), "~1h05");
        assert_eq!(human_duration(19), "0:19");
        assert_eq!(human_duration(225), "3:45");
        assert_eq!(human_duration(3723), "1:02:03");
    }

    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn progress_line_fits_width_and_handles_unknown_total() {
        let p = item(Phase::Downloading, None).progress;
        assert_eq!(line_text(&progress_line(&p, 63)).chars().count(), 63);
        // Sem espaço pra barra: fica só rótulo e número (o resto o terminal corta).
        assert_eq!(
            line_text(&progress_line(&p, 10)),
            "baixando vídeo  64% · 2.1 MB/s"
        );
        let unknown = Progress {
            downloaded: 500_000,
            audio: true,
            ..Progress::default()
        };
        assert_eq!(line_text(&progress_line(&unknown, 10)), "baixando áudio  …");
        assert_eq!(remaining_text(&unknown), "500 KB baixados");
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
