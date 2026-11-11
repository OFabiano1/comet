#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    YouTube,
    Twitter,
    TikTok,
}

impl Site {
    pub fn label(self) -> &'static str {
        match self {
            Site::YouTube => "youtube",
            Site::Twitter => "twitter",
            Site::TikTok => "tiktok",
        }
    }
}

pub fn detect(raw: &str) -> Option<Site> {
    let url = raw.trim().to_ascii_lowercase();
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(&url);
    let (host, path) = match without_scheme.split_once('/') {
        Some((h, p)) => (h, p),
        None => (without_scheme, ""),
    };
    match host.strip_prefix("www.") {
        Some(h) => host_from(h, path),
        None => host_from(host, path),
    }
}

fn host_from(host: &str, path: &str) -> Option<Site> {
    match host {
        // só link de vídeo: canal ou playlist baixaria tudo
        "youtube.com" | "m.youtube.com"
            if ["watch?", "shorts/", "live/"].iter().any(|p| path.starts_with(p)) =>
        {
            Some(Site::YouTube)
        }
        "youtu.be" if !path.is_empty() => Some(Site::YouTube),
        "x.com" | "twitter.com" if path.contains("status/") => Some(Site::Twitter),
        "tiktok.com" if path.contains("video/") || path.contains("photo/") => Some(Site::TikTok),
        "vm.tiktok.com" => Some(Site::TikTok),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_youtube() {
        assert_eq!(
            detect("  https://www.youtube.com/watch?v=abc \n"),
            Some(Site::YouTube)
        );
        assert_eq!(detect("https://youtu.be/abc123"), Some(Site::YouTube));
    }

    #[test]
    fn detects_twitter_video_and_x() {
        assert_eq!(detect("https://x.com/user/status/123"), Some(Site::Twitter));
        assert_eq!(
            detect("https://twitter.com/user/status/123"),
            Some(Site::Twitter)
        );
    }

    #[test]
    fn detects_tiktok() {
        assert_eq!(
            detect("https://www.tiktok.com/@user/video/123"),
            Some(Site::TikTok)
        );
    }

    #[test]
    fn rejects_non_media_urls() {
        assert_eq!(detect("https://example.com"), None);
        assert_eq!(detect(""), None);
        assert_eq!(detect("https://x.com/user"), None);
        assert_eq!(detect("https://www.tiktok.com/@user"), None);
    }

    #[test]
    fn rejects_youtube_channels_and_playlists() {
        assert_eq!(detect("https://www.youtube.com/@canal"), None);
        assert_eq!(detect("https://www.youtube.com/@canal/videos"), None);
        assert_eq!(detect("https://www.youtube.com/playlist?list=PL1"), None);
        assert_eq!(detect("https://youtube.com/channel/UC1"), None);
        assert_eq!(detect("https://youtu.be/"), None);
        assert_eq!(detect("https://youtube.com/shorts/abc"), Some(Site::YouTube));
        assert_eq!(detect("https://m.youtube.com/live/abc"), Some(Site::YouTube));
    }
}
