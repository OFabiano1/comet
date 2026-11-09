use crate::app::{App, Item, Kind, Phase};
use crate::theme::{ACCENT, ACCENT_LIGHT, BG, SURFACE_ALT};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

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

fn dim() -> Style {
    Style::new().fg(SURFACE_ALT)
}

fn accent() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

fn phase_chip(item: &Item) -> (String, Style) {
    match item.phase {
        Phase::Queued => ("QUEUED".to_string(), dim()),
        Phase::Downloading => ("DOWNLOADING".to_string(), Style::new().fg(ACCENT_LIGHT)),
        Phase::Converting => ("CONVERTING".to_string(), Style::new().fg(ACCENT_LIGHT)),
        Phase::Done => ("DONE".to_string(), Style::new().fg(ACCENT_LIGHT)),
        Phase::Failed => (
            format!("FAILED: {}", item.error.clone().unwrap_or_default()),
            Style::new().fg(Color::Red),
        ),
    }
}

fn ext_of(item: &Item) -> &'static str {
    match item.kind {
        Kind::Video => "mp4",
        Kind::Gif => "gif",
    }
}

/// 64% · 12.4 MB / 19.3 MB — line shown right of title while downloading.
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
    f.buffer_mut().set_style(area, Style::new().bg(BG));

    let w = area.width as usize;
    let compact = area.height < 14;
    let detection: Vec<(String, Style)> = match crate::sites::detect(&app.input) {
        Some(site) => vec![
            (format!("{} DETECTED", site.label().to_uppercase()), dim()),
            (" · ".to_string(), dim()),
            ("VALID URL".to_string(), Style::new().fg(ACCENT_LIGHT)),
        ],
        None if app.input.trim().is_empty() => {
            vec![("COLE UMA URL E APERTE ENTER".to_string(), dim())]
        }
        None => vec![("INVALID URL".to_string(), Style::new().fg(Color::Red))],
    };

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled("☄ COMET", accent())),
        Line::from(Span::styled(
            app.status.clone(),
            Style::new().fg(ACCENT_LIGHT),
        )),
    ];
    if !compact {
        lines.push(Line::from(Span::styled("VIDEO DOWNLOADER · v1.0.0", dim())));
        lines.push(Line::default());
    }
    lines.push(Line::from(Span::styled("DOWNLOAD URL", dim())));
    lines.push(Line::from(Span::styled(
        format!("> {}", app.input),
        Style::new().fg(Color::White),
    )));
    lines.push(Line::from(
        detection
            .into_iter()
            .map(|(t, s)| Span::styled(t, s))
            .collect::<Vec<_>>(),
    ));
    if !compact {
        lines.push(Line::default());
    }
    lines.push(Line::from(Span::styled("DOWNLOAD QUEUE", dim())));
    let queue_top = lines.len() as u16;

    let header_lines = lines; // header + input + detection + queue label
    // Items go in their own rect so the footer can never clobber them.
    let mut item_lines: Vec<Line> = Vec::new();
    let footer_h = 2u16;
    let queue_h = area.height.saturating_sub(queue_top + footer_h) as usize;
    let visible = (queue_h / 3).max(1);
    let sel = app.selected.min(app.items.len().saturating_sub(1));
    let offset = sel.saturating_sub(visible - 1);

    for (i, item) in app.items.iter().enumerate().skip(offset).take(visible) {
        let marker = if i == sel { "▎" } else { " " };
        let title_style = if i == sel {
            Style::new().fg(Color::White)
        } else {
            Style::new().fg(SURFACE_ALT)
        };
        let (chip, chip_style) = phase_chip(item);
        let meta = format!("{} · {}", item.site.label(), ext_of(item));
        let right = if item.phase == Phase::Downloading {
            progress_text(item)
        } else {
            String::new()
        };
        let title_max = w.saturating_sub(2);
        let right_pad = if right.is_empty() {
            0
        } else {
            right.chars().count() + 1
        };
        item_lines.push(Line::from(vec![
            Span::styled(marker, Style::new().fg(ACCENT)),
            Span::styled(
                truncate_ellipsis(&item.title, title_max.saturating_sub(right_pad)),
                title_style,
            ),
            Span::styled(format!(" {right}"), Style::new().fg(ACCENT_LIGHT)),
        ]));
        item_lines.push(Line::from(Span::styled(format!("  {meta}"), dim())));
        item_lines.push(Line::from(Span::styled(format!("  {chip}"), chip_style)));
    }

    f.render_widget(
        Paragraph::new(header_lines),
        ratatui::layout::Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: queue_top,
        },
    );
    f.render_widget(
        Paragraph::new(item_lines),
        ratatui::layout::Rect {
            x: area.x,
            y: area.y + queue_top,
            width: area.width,
            height: queue_h as u16,
        },
    );
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(app.summary(), dim())),
            Line::from(Span::styled(
                "o OPEN FOLDER · ENTER ADD · D DOWNLOAD · G GIF · Q QUIT",
                dim(),
            )),
        ]),
        ratatui::layout::Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(footer_h),
            width: area.width,
            height: footer_h,
        },
    );
    // Cursor sits at the end of the input row.
    let input_row = queue_top.saturating_sub(2);
    let cur = (2 + app.input.chars().count() as u16).min(w as u16 - 1);
    f.set_cursor_position((cur, input_row));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Kind, Phase};
    use crate::sites::Site;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn truncate_handles_edge_cases() {
        assert_eq!(truncate_ellipsis("oi", 5), "oi");
        assert_eq!(truncate_ellipsis("hello world", 5), "hell…");
        assert_eq!(truncate_ellipsis("hello", 5), "hello");
        assert_eq!(truncate_ellipsis("hello", 0), "");
    }

    #[test]
    fn draws_header_input_and_queue_in_tiny_terminal() {
        let mut app = App::default();
        app.input = "https://x.com/u/status/1".to_string();
        app.items.push(Item {
            url: "https://x.com/u/status/1".to_string(),
            title: "video title goes here".to_string(),
            site: Site::Twitter,
            kind: Kind::Video,
            phase: Phase::Downloading,
            pct: 64.0,
            size_str: "19.3 MB".to_string(),
            got_str: "12.4 MB".to_string(),
            error: None,
        });

        let backend = TestBackend::new(40, 18);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &app)).unwrap(); // must not panic
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("☄ COMET"));
        assert!(text.contains("TWITTER DETECTED"));
        assert!(text.contains("64%"));

        // 20x8 must not panic; queue may be clipped, footer never clobbers input.
        let mut tiny = Terminal::new(TestBackend::new(20, 8)).unwrap();
        tiny.draw(|f| draw(f, &app)).unwrap();
    }
}
