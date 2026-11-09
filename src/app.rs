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
    pub pct: f64,
    pub size_str: String,
    pub got_str: String,
    pub error: Option<String>,
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

impl Default for App {
    fn default() -> Self {
        Self {
            input: String::new(),
            status: "READY".to_string(),
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
            return Err("INVALID URL".to_string());
        };
        if self.items.iter().any(|i| i.url == raw) {
            return Err("ALREADY IN QUEUE".to_string());
        }
        self.items.push(Item {
            url: raw,
            title: "…".to_string(),
            site,
            kind: Kind::Video,
            phase: Phase::Queued,
            pct: 0.0,
            size_str: String::new(),
            got_str: String::new(),
            error: None,
        });
        self.selected = self.items.len() - 1;
        self.input.clear();
        Ok(())
    }

    pub fn toggle_gif(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            if item.phase == Phase::Queued {
                item.kind = match item.kind {
                    Kind::Video => Kind::Gif,
                    Kind::Gif => Kind::Video,
                };
            }
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{} ITEM{} · {} COMPLETED",
            self.items.len(),
            if self.items.len() == 1 { "" } else { "S" },
            self.completed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yt_url() -> String {
        "https://www.youtube.com/watch?v=abc".to_string()
    }

    #[test]
    fn add_url_pushes_item_and_clears_input() {
        let mut app = App::default();
        app.input = yt_url();
        app.add_url().unwrap();
        assert_eq!(app.items.len(), 1);
        assert_eq!(app.items[0].site, Site::YouTube);
        assert_eq!(app.items[0].phase, Phase::Queued);
        assert_eq!(app.items[0].kind, Kind::Video);
        assert!(app.input.is_empty());
    }

    #[test]
    fn add_url_invalid_keeps_input() {
        let mut app = App::default();
        app.input = "https://example.com".to_string();
        assert_eq!(app.add_url(), Err("INVALID URL".to_string()));
        assert_eq!(app.input, "https://example.com");
        assert!(app.items.is_empty());
    }

    #[test]
    fn add_url_rejects_duplicate() {
        let mut app = App::default();
        app.input = yt_url();
        app.add_url().unwrap();
        app.input = yt_url();
        assert_eq!(app.add_url(), Err("ALREADY IN QUEUE".to_string()));
        assert_eq!(app.items.len(), 1);
    }

    #[test]
    fn toggle_gif_only_when_queued() {
        let mut app = App::default();
        app.input = yt_url();
        app.add_url().unwrap();
        app.toggle_gif();
        assert_eq!(app.items[0].kind, Kind::Gif);
        app.items[0].phase = Phase::Downloading;
        app.toggle_gif();
        assert_eq!(app.items[0].kind, Kind::Gif);
    }

    #[test]
    fn summary_plural_and_singular() {
        let mut app = App::default();
        app.input = yt_url();
        app.add_url().unwrap();
        app.input = "https://x.com/u/status/1".to_string();
        app.add_url().unwrap();
        app.completed = 1;
        assert_eq!(app.summary(), "2 ITEMS · 1 COMPLETED");
        app.completed = 0;
        app.items.pop();
        assert_eq!(app.summary(), "1 ITEM · 0 COMPLETED");
    }

    #[test]
    fn site_labels_lowercase() {
        assert_eq!(Site::YouTube.label(), "youtube");
        assert_eq!(Site::Twitter.label(), "twitter");
        assert_eq!(Site::TikTok.label(), "tiktok");
    }
}
