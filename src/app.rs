use crate::sites::{self, Site};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Queued,
    Downloading,
    Converting,
    Done,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Video,
    Gif,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub url: String,
    pub title: String,
    pub site: Site,
    pub kind: Kind,
    pub phase: Phase,
    pub info: VideoInfo,
    pub progress: Progress,
    pub error: Option<String>,
}

/// Dados do vídeo que o yt-dlp devolve antes de baixar.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VideoInfo {
    pub channel: String,
    pub duration_secs: Option<u64>,
}

/// Estado do download da parte atual (vídeo e áudio baixam separados).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
    /// bytes por segundo
    pub speed: Option<f64>,
    pub eta_secs: Option<u64>,
    pub audio: bool,
}

impl Progress {
    pub fn pct(&self) -> Option<f64> {
        let total = self.total.filter(|t| *t > 0)?;
        Some((self.downloaded as f64 / total as f64 * 100.0).min(100.0))
    }

    pub fn remaining(&self) -> Option<u64> {
        self.total.map(|t| t.saturating_sub(self.downloaded))
    }
}

pub struct App {
    pub input: String,
    pub status: String,
    pub items: Vec<Item>,
    pub selected: usize,
    pub completed: usize,
    pub busy: bool,
    pub should_quit: bool,
}

pub const STATUS_READY: &str = "pronto";
pub const STATUS_BUSY: &str = "baixando a fila";

impl Default for App {
    fn default() -> Self {
        Self {
            input: String::new(),
            status: STATUS_READY.to_string(),
            items: Vec::new(),
            selected: 0,
            completed: 0,
            busy: false,
            should_quit: false,
        }
    }
}

impl App {
    pub fn add_url(&mut self) -> Result<(), String> {
        let raw = self.input.trim().to_string();
        let Some(site) = sites::detect(&raw) else {
            return Err("link não reconhecido".to_string());
        };
        if self.items.iter().any(|i| i.url == raw) {
            return Err("esse link já está na fila".to_string());
        }
        self.items.push(Item {
            url: raw,
            title: "…".to_string(),
            site,
            kind: Kind::Video,
            phase: Phase::Queued,
            info: VideoInfo::default(),
            progress: Progress::default(),
            error: None,
        });
        self.selected = self.items.len() - 1;
        self.input.clear();
        Ok(())
    }

    pub fn toggle_gif(&mut self) {
        let selected = self.items.get_mut(self.selected);
        if let Some(item) = selected.filter(|i| i.phase == Phase::Queued) {
            item.kind = match item.kind {
                Kind::Video => Kind::Gif,
                Kind::Gif => Kind::Video,
            };
        }
    }

    /// "2 itens · 1 baixado"; sem nada baixado, só a contagem.
    pub fn summary(&self) -> String {
        let n = self.items.len();
        let itens = format!("{n} {}", if n == 1 { "item" } else { "itens" });
        match self.completed {
            0 => itens,
            1 => format!("{itens} · 1 baixado"),
            c => format!("{itens} · {c} baixados"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yt_url() -> String {
        "https://www.youtube.com/watch?v=abc".to_string()
    }

    fn app_with(input: &str) -> App {
        App {
            input: input.to_string(),
            ..App::default()
        }
    }

    #[test]
    fn add_url_pushes_item_and_clears_input() {
        let mut app = app_with(&yt_url());
        app.add_url().unwrap();
        assert_eq!(app.items.len(), 1);
        assert_eq!(app.items[0].site, Site::YouTube);
        assert_eq!(app.items[0].phase, Phase::Queued);
        assert_eq!(app.items[0].kind, Kind::Video);
        assert!(app.input.is_empty());
    }

    #[test]
    fn add_url_invalid_keeps_input() {
        let mut app = app_with("https://example.com");
        assert_eq!(app.add_url(), Err("link não reconhecido".to_string()));
        assert_eq!(app.input, "https://example.com");
        assert!(app.items.is_empty());
    }

    #[test]
    fn add_url_rejects_duplicate() {
        let mut app = app_with(&yt_url());
        app.add_url().unwrap();
        app.input = yt_url();
        assert_eq!(app.add_url(), Err("esse link já está na fila".to_string()));
        assert_eq!(app.items.len(), 1);
    }

    #[test]
    fn toggle_gif_only_when_queued() {
        let mut app = app_with(&yt_url());
        app.add_url().unwrap();
        app.toggle_gif();
        assert_eq!(app.items[0].kind, Kind::Gif);
        app.items[0].phase = Phase::Downloading;
        app.toggle_gif();
        assert_eq!(app.items[0].kind, Kind::Gif);
    }

    #[test]
    fn summary_plural_and_singular() {
        let mut app = app_with(&yt_url());
        assert_eq!(app.summary(), "0 itens");
        app.add_url().unwrap();
        assert_eq!(app.summary(), "1 item");
        app.input = "https://x.com/u/status/1".to_string();
        app.add_url().unwrap();
        app.completed = 1;
        assert_eq!(app.summary(), "2 itens · 1 baixado");
        app.completed = 2;
        assert_eq!(app.summary(), "2 itens · 2 baixados");
    }

    #[test]
    fn site_labels_lowercase() {
        assert_eq!(Site::YouTube.label(), "youtube");
        assert_eq!(Site::Twitter.label(), "twitter");
        assert_eq!(Site::TikTok.label(), "tiktok");
    }
}
