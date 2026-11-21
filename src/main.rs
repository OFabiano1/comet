mod app;
mod downloader;
mod sites;
mod theme;
mod ui;
mod vendor;

use anyhow::Result;
use app::{App, Phase};
use crossterm::event::{self, EnableBracketedPaste, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::collections::VecDeque;
use std::io::{stdout, Stdout};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

struct Terminal {
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>>,
}

impl Terminal {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        crossterm::execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
        let backend = ratatui::backend::CrosstermBackend::new(stdout);
        Ok(Self {
            terminal: ratatui::Terminal::new(backend)?,
        })
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        use crossterm::event::DisableBracketedPaste;
        let _ = crossterm::execute!(
            self.terminal.backend_mut(),
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

async fn run_queue(app: Arc<Mutex<App>>) {
    let _guard = downloader::BusyGuard { app: app.clone() };
    let (ytdlp, ffmpeg) = match vendor::ensure_binaries(&app).await {
        Ok(b) => b,
        Err(e) => {
            let mut a = app.lock().await;
            a.status = e.to_string();
            a.busy = false;
            return;
        }
    };
    loop {
        let next = {
            let a = app.lock().await;
            a.items.iter().position(|i| i.phase == Phase::Queued)
        };
        let Some(idx) = next else { break };
        match downloader::download(&app, idx, &ytdlp, &ffmpeg).await {
            Err(e) => {
                let mut a = app.lock().await;
                let item = &mut a.items[idx];
                item.phase = Phase::Failed;
                item.error = Some(e.to_string());
            }
            Ok(()) => {
                let mut a = app.lock().await;
                let item = a.items[idx].clone();
                if item.phase == Phase::Done && item.saved.is_some() {
                    let entry = downloader::record_history(&item);
                    a.history.insert(0, entry);
                }
            }
        }
    }
    let mut a = app.lock().await;
    a.status = app::STATUS_READY.to_string();
    a.busy = false;
}

/// Com arquivo: abre a pasta já com ele selecionado. Sem: abre a pasta de downloads.
fn open_in_explorer(file: Option<&std::path::Path>, dir: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new("explorer");
    match file {
        // explorer não aceita o /select com aspas no lugar que o Rust põe: vai cru.
        Some(f) => cmd.raw_arg(format!("/select,\"{}\"", f.display())),
        None => {
            let _ = std::fs::create_dir_all(dir);
            cmd.arg(dir)
        }
    };
    let _ = cmd.spawn();
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut terminal = Terminal::new()?;
    let app = Arc::new(Mutex::new(App {
        history: downloader::load_history(),
        ..App::default()
    }));
    let mut pending: VecDeque<Event> = VecDeque::new();
    let mut pasting = false;

    while !app.lock().await.should_quit {
        {
            let a = app.lock().await;
            terminal.terminal.draw(|f| ui::draw(f, &a))?;
        }
        if pending.is_empty() {
            if !event::poll(Duration::from_millis(16))? {
                continue;
            }
            // Pega tudo que já chegou junto: é assim que dá pra reconhecer uma colagem.
            pending.push_back(event::read()?);
            while event::poll(Duration::ZERO)? {
                pending.push_back(event::read()?);
            }
            pasting = is_paste_burst(pending.make_contiguous());
        }
        let Some(ev) = pending.pop_front() else { continue };
        match ev {
            Event::Paste(text) => {
                app.lock().await.input = text.trim().to_string();
            }
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let mut a = app.lock().await;
                // AltGr chega como Ctrl+Alt e digita caractere de verdade; atalho é só um dos dois.
                let chord = key.modifiers.contains(KeyModifiers::CONTROL)
                    != key.modifiers.contains(KeyModifiers::ALT);
                // Atalho de uma letra só com o campo vazio e fora de colagem.
                let hotkey = a.input.is_empty() && !pasting;
                match key.code {
                    KeyCode::Esc => a.should_quit = true,
                    // raw mode: Ctrl+C não manda sinal, chega como tecla.
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        a.should_quit = true
                    }
                    // Atalho com Ctrl/Alt (Ctrl+V sem bracketed paste) não vira letra no campo.
                    KeyCode::Char(_) if chord => {}
                    KeyCode::Char('d') | KeyCode::Char('D') if hotkey && !a.busy => {
                        a.busy = true;
                        a.status = app::STATUS_BUSY.to_string();
                        let handle = app.clone();
                        tokio::spawn(async move { run_queue(handle).await });
                    }
                    KeyCode::Char('g') | KeyCode::Char('G') if hotkey => a.toggle_gif(),
                    KeyCode::Char('o') | KeyCode::Char('O') if hotkey => {
                        let file = a.items.get(a.selected).and_then(|i| i.saved.clone());
                        open_in_explorer(file.as_deref().filter(|f| f.is_file()), &a.out_dir);
                    }
                    KeyCode::Char('h') | KeyCode::Char('H') if hotkey => {
                        a.toggle_history()
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') if hotkey && !a.busy => {
                        a.should_quit = true
                    }
                    KeyCode::Char(c) => a.input.push(c),
                    KeyCode::Backspace => {
                        a.input.pop();
                    }
                    // No histórico, enter põe o link de volta na fila e baixa.
                    KeyCode::Enter if a.show_history => {
                        a.input = a
                            .history
                            .get(a.history_sel)
                            .map(|h| h.url.clone())
                            .unwrap_or_default();
                        match a.add_url() {
                            Ok(()) => {
                                a.show_history = false;
                                a.status = app::STATUS_READY.to_string();
                                if !a.busy {
                                    a.busy = true;
                                    a.status = app::STATUS_BUSY.to_string();
                                    let handle = app.clone();
                                    tokio::spawn(async move { run_queue(handle).await });
                                }
                            }
                            Err(e) => {
                                a.input.clear();
                                a.status = e;
                            }
                        }
                    }
                    KeyCode::Enter => match a.add_url() {
                        Ok(()) => a.status = app::STATUS_READY.to_string(),
                        Err(e) => a.status = e,
                    },
                    // Seta de cima no topo da fila abre o histórico; dentro dele, sobe na lista.
                    KeyCode::Up if a.show_history => {
                        a.history_sel = a.history_sel.saturating_sub(1);
                    }
                    KeyCode::Up if a.input.is_empty() && a.selected == 0 && !a.history.is_empty() => {
                        a.show_history = true;
                        a.history_sel = 0;
                    }
                    KeyCode::Up => {
                        a.selected = a.selected.saturating_sub(1);
                    }
                    KeyCode::Down if a.show_history => {
                        if a.history_sel + 1 < a.history.len() {
                            a.history_sel += 1;
                        }
                    }
                    KeyCode::Down if a.selected + 1 < a.items.len() => a.selected += 1,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Windows não manda Event::Paste: colar chega como teclas em rajada. Mais de uma letra
/// já esperando = colagem; sem isso o "h" de "https" abria o histórico.
fn is_paste_burst(events: &[Event]) -> bool {
    events
        .iter()
        .filter(|e| {
            matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && matches!(k.code, KeyCode::Char(_)))
        })
        .count()
        > 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn key(c: char, kind: KeyEventKind) -> Event {
        Event::Key(KeyEvent::new_with_kind(KeyCode::Char(c), KeyModifiers::NONE, kind))
    }

    #[test]
    fn pasted_link_is_text_but_single_key_is_hotkey() {
        let press = KeyEventKind::Press;
        let release = KeyEventKind::Release;
        // Uma tecla só, com o solta junto: atalho.
        assert!(!is_paste_burst(&[key('h', press), key('h', release)]));
        // "https://…" colado: rajada de letras.
        let pasted: Vec<Event> = "https://x.com".chars().map(|c| key(c, press)).collect();
        assert!(is_paste_burst(&pasted));
        assert!(!is_paste_burst(&[]));
    }
}
