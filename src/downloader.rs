use crate::app::{App, Kind, Phase};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Progress {
    pub pct: f64,
    pub size_str: String,
    pub got_str: String,
}

pub fn parse_title(line: &str) -> Option<String> {
    line.strip_prefix("TITLE:")
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// yt-dlp precisa de cookies do navegador para Twitter/X e TikTok em 2026.
pub fn first_installed(cands: &[(String, PathBuf)]) -> Option<String> {
    cands
        .iter()
        .find(|(_, p)| p.is_file())
        .map(|(name, _)| name.clone())
}

pub fn browser_cookie_args() -> Vec<String> {
    let pf = std::env::var("ProgramFiles").unwrap_or_default();
    let pf86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let cands: Vec<(String, PathBuf)> = vec![
        (
            "chrome".to_string(),
            PathBuf::from(&pf).join("Google/Chrome/Application/chrome.exe"),
        ),
        (
            "chrome".to_string(),
            PathBuf::from(&pf86).join("Google/Chrome/Application/chrome.exe"),
        ),
        (
            "edge".to_string(),
            PathBuf::from(&pf86).join("Microsoft/Edge/Application/msedge.exe"),
        ),
        (
            "edge".to_string(),
            PathBuf::from(&pf).join("Microsoft/Edge/Application/msedge.exe"),
        ),
        (
            "brave".to_string(),
            PathBuf::from(&pf).join("BraveSoftware/Brave-Browser/Application/brave.exe"),
        ),
    ];
    let _ = &local;
    match first_installed(&cands) {
        Some(name) => vec!["--cookies-from-browser".to_string(), name],
        None => Vec::new(),
    }
}

/// Garante que `busy` volte a false mesmo se a task de download entrar em panic.
pub struct BusyGuard {
    pub app: Arc<Mutex<App>>,
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        let app = self.app.clone();
        tokio::spawn(async move {
            let mut a = app.lock().await;
            a.busy = false;
        });
    }
}

pub fn parse_progress(line: &str) -> Option<Progress> {
    let rest = line.strip_prefix("[PROG]")?.trim();
    let mut parts = rest.split('|');
    let pct: f64 = parts
        .next()?
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse()
        .ok()?;
    let size_str = parts.next()?.trim().to_string();
    let got_str = parts.next()?.trim().to_string();
    Some(Progress {
        pct,
        size_str,
        got_str,
    })
}

pub const GIF_FAILED: &str = "gif falhou, o mp4 ficou salvo";

pub fn parse_error(stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    // "age" solto casaria com "webpage", "image", "message".
    let login_wall = [
        "private",
        "sign in",
        "login",
        "confirm your age",
        "age-restricted",
        "age restricted",
        "members-only",
        "authenticate",
    ];
    if login_wall.iter().any(|k| lower.contains(k)) {
        "vídeo privado ou pede login".to_string()
    } else if lower.contains("unsupported url") {
        "link não suportado".to_string()
    } else {
        // A linha do ERROR diz o motivo; WARNING antes dela é ruído.
        let reason = stderr
            .lines()
            .find_map(|l| l.split_once("ERROR:").map(|(_, r)| r))
            .or_else(|| stderr.lines().rev().find(|l| !l.trim().is_empty()));
        match reason {
            Some(r) => truncate_msg(r),
            None => "o download falhou".to_string(),
        }
    }
}

fn truncate_msg(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() <= 48 {
        t.to_string()
    } else {
        t.chars().take(47).collect::<String>() + "…"
    }
}

pub fn extract_final_path(line: &str) -> Option<String> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('[') || t.contains(' ') {
        return None;
    }
    let looks_like_path = t.contains('\\') || t.contains('/');
    let looks_like_media = [".mp4", ".webm", ".mkv", ".mov", ".gif"]
        .iter()
        .any(|ext| t.to_ascii_lowercase().ends_with(ext));
    if looks_like_path || looks_like_media {
        Some(t.to_string())
    } else {
        None
    }
}

pub fn downloads_dir() -> PathBuf {
    dirs::download_dir()
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .join("Comet")
}

pub async fn download(
    app: &Arc<Mutex<App>>,
    idx: usize,
    ytdlp: &Path,
    _ffmpeg: &Path,
) -> Result<()> {
    let (url, out_dir) = {
        let mut a = app.lock().await;
        let item = &mut a.items[idx];
        item.phase = Phase::Downloading;
        item.pct = 0.0;
        (item.url.clone(), downloads_dir())
    };
    tokio::fs::create_dir_all(&out_dir).await?;

    let mut args: Vec<String> = vec![
        "--newline".to_string(),
        "--no-playlist".to_string(),
        "--merge-output-format".to_string(),
        "mp4".to_string(),
        "--progress-template".to_string(),
        "download:[PROG]%(progress._percent_str)s|%(progress._total_bytes_estimate_str)s|%(progress._downloaded_bytes_str)s".to_string(),
        "--print".to_string(),
        "after_move:filepath".to_string(),
        "--print".to_string(),
        "TITLE:%(title)s".to_string(),
        "-o".to_string(),
        "%(title).80B.%(ext)s".to_string(),
    ];
    args.extend(browser_cookie_args());
    args.push(url.clone());

    let mut child = tokio::process::Command::new(ytdlp)
        .args(&args)
        .current_dir(&out_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        // Esc no meio do download não deixa yt-dlp órfão rodando.
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let app2 = app.clone();
    let out_task = tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut final_path = None;
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let l = line.trim_end();
                    if let Some(p) = parse_progress(l) {
                        let mut a = app2.lock().await;
                        let item = &mut a.items[idx];
                        item.pct = p.pct;
                        item.size_str = p.size_str;
                        item.got_str = p.got_str;
                    } else if let Some(title) = parse_title(l) {
                        let mut a = app2.lock().await;
                        a.items[idx].title = title;
                    } else if let Some(path) = extract_final_path(l) {
                        final_path = Some(path);
                    }
                }
            }
        }
        final_path
    });
    let err_task = tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stderr);
        let mut tail: Vec<String> = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    tail.push(line.trim_end().to_string());
                    if tail.len() > 12 {
                        tail.remove(0);
                    }
                }
            }
        }
        tail
    });

    let final_path = out_task.await.unwrap_or(None);
    let err_tail = err_task.await.unwrap_or_default();
    let status = child.wait().await?;

    let (kind, ok) = {
        let mut a = app.lock().await;
        if status.success() {
            a.completed += 1;
            (a.items[idx].kind, true)
        } else {
            let item = &mut a.items[idx];
            item.phase = Phase::Failed;
            item.error = Some(parse_error(&err_tail.join("\n")));
            (item.kind, false)
        }
    };
    if ok && kind == Kind::Gif {
        if let Some(mp4) = final_path {
            to_gif(app, idx, _ffmpeg, Path::new(&mp4)).await?;
        } else {
            let mut a = app.lock().await;
            a.items[idx].phase = Phase::Done;
            a.items[idx].error = Some(GIF_FAILED.to_string());
        }
    } else if ok {
        app.lock().await.items[idx].phase = Phase::Done;
    }
    Ok(())
}

pub fn palette_args(input: &Path, png: &Path) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-vf".to_string(),
        "palettegen".to_string(),
        png.to_string_lossy().to_string(),
    ]
}

pub fn gif_args(input: &Path, png: &Path, out: &Path) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-i".to_string(),
        png.to_string_lossy().to_string(),
        "-lavfi".to_string(),
        "paletteuse".to_string(),
        out.to_string_lossy().to_string(),
    ]
}

async fn run_ffmpeg(ffmpeg: &Path, args: &[String]) -> Result<()> {
    let out = tokio::process::Command::new(ffmpeg)
        .args(args)
        .output()
        .await?;
    if out.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("ffmpeg failed"))
    }
}

pub async fn to_gif(app: &Arc<Mutex<App>>, idx: usize, ffmpeg: &Path, mp4: &Path) -> Result<()> {
    app.lock().await.items[idx].phase = Phase::Converting;
    let png = mp4.with_extension("png");
    let gif = mp4.with_extension("gif");
    let conv = async {
        run_ffmpeg(ffmpeg, &palette_args(mp4, &png)).await?;
        run_ffmpeg(ffmpeg, &gif_args(mp4, &png, &gif)).await?;
        // shortcut: gif overwrite fallback omitted; names always dedupe via yt-dlp template
        tokio::fs::remove_file(&png).await?;
        tokio::fs::remove_file(mp4).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    let mut a = app.lock().await;
    let item = &mut a.items[idx];
    match conv {
        Ok(()) => item.phase = Phase::Done,
        Err(_) => {
            item.phase = Phase::Done;
            item.error = Some(GIF_FAILED.to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_template_lines() {
        let p = parse_progress("[PROG]  42.3%|19.30MiB|8.16MiB").unwrap();
        assert_eq!(p.pct, 42.3);
        assert_eq!(p.size_str, "19.30MiB");
        assert_eq!(p.got_str, "8.16MiB");
        let p = parse_progress("[PROG] 100.0%|2.00MiB|2.00MiB").unwrap();
        assert_eq!(p.pct, 100.0);
        assert!(parse_progress("garbage").is_none());
        assert!(parse_progress("[download] Destination: x.mp4").is_none());
    }

    #[test]
    fn maps_login_wall_and_private_errors() {
        assert_eq!(
            parse_error("ERROR: This video is private. [youtube]"),
            "vídeo privado ou pede login"
        );
        assert_eq!(
            parse_error("ERROR: Sign in to confirm your age"),
            "vídeo privado ou pede login"
        );
        assert_eq!(
            parse_error("ERROR: Unsupported URL: https://example.com"),
            "link não suportado"
        );
    }

    #[test]
    fn generic_error_uses_the_error_line_not_a_word_inside_it() {
        // "webpage" contém "age": antes virava "privado ou pede login".
        let stderr =
            "WARNING: [youtube] falling back\nERROR: Unable to download webpage: HTTP Error 404";
        assert_eq!(
            parse_error(stderr),
            "Unable to download webpage: HTTP Error 404"
        );
        assert_eq!(parse_error("algo estranho\n"), "algo estranho");
        assert_eq!(parse_error(""), "o download falhou");
    }

    #[test]
    fn parses_title_line() {
        assert_eq!(
            parse_title("TITLE:Me at the zoo"),
            Some("Me at the zoo".to_string())
        );
        assert_eq!(parse_title("[download] 100% of 2MiB"), None);
    }

    #[test]
    fn picks_first_installed_browser() {
        let dir = std::env::temp_dir().join("comet-browser-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("chrome.exe");
        let present = dir.join("msedge.exe");
        std::fs::write(&present, b"x").unwrap();
        let cands = vec![
            ("chrome".to_string(), missing),
            ("edge".to_string(), present),
        ];
        assert_eq!(first_installed(&cands), Some("edge".to_string()));
        let none = vec![("chrome".to_string(), dir.join("nope.exe"))];
        assert_eq!(first_installed(&none), None);
    }

    #[tokio::test]
    async fn busy_guard_resets_busy_on_drop() {
        let app = Arc::new(Mutex::new(App::default()));
        {
            let mut a = app.lock().await;
            a.busy = true;
        }
        {
            let _guard = BusyGuard { app: app.clone() };
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!app.lock().await.busy);
    }

    #[test]
    fn extracts_final_filepath_line() {
        assert_eq!(
            extract_final_path("C:\\Users\\F\\Downloads\\Comet\\video.mp4"),
            Some("C:\\Users\\F\\Downloads\\Comet\\video.mp4".to_string())
        );
        assert_eq!(extract_final_path("[download] 100% of 2MiB"), None);
        assert_eq!(extract_final_path("[PROG] 50%|2MiB|1MiB"), None);
        assert_eq!(extract_final_path("has already been downloaded"), None);
    }

    #[test]
    fn palette_and_gif_args_exact() {
        let png = Path::new("C:\\t\\p.png");
        let mp4 = Path::new("C:\\t\\v.mp4");
        let gif = Path::new("C:\\t\\v.gif");
        assert_eq!(
            palette_args(mp4, png),
            vec![
                "-y".to_string(),
                "-i".to_string(),
                mp4.to_string_lossy().to_string(),
                "-vf".to_string(),
                "palettegen".to_string(),
                png.to_string_lossy().to_string(),
            ]
        );
        assert_eq!(
            gif_args(mp4, png, gif),
            vec![
                "-y".to_string(),
                "-i".to_string(),
                mp4.to_string_lossy().to_string(),
                "-i".to_string(),
                png.to_string_lossy().to_string(),
                "-lavfi".to_string(),
                "paletteuse".to_string(),
                gif.to_string_lossy().to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn to_gif_converts_and_keeps_nothing() {
        let ffmpeg = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/ffmpeg.exe"));
        if !ffmpeg.is_file() {
            return; // vendor ausente: teste real coberto no bloco de rede
        }
        let dir = std::env::temp_dir().join("comet-gif-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mp4 = dir.join("src.mp4");
        let status = std::process::Command::new(ffmpeg)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=1:size=128x128",
                mp4.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(status.success());

        let app = Arc::new(Mutex::new(App::default()));
        app.lock().await.items.push(crate::app::Item {
            url: "x".into(),
            title: "src".into(),
            site: crate::sites::Site::Twitter,
            kind: Kind::Gif,
            phase: Phase::Converting,
            pct: 100.0,
            size_str: String::new(),
            got_str: String::new(),
            error: None,
        });
        to_gif(&app, 0, ffmpeg, &mp4).await.unwrap();
        assert!(dir.join("src.gif").is_file());
        assert!(!mp4.exists());
        assert!(!dir.join("src.png").exists());
        let a = app.lock().await;
        assert_eq!(a.items[0].phase, Phase::Done);
        assert!(a.items[0].error.is_none());
    }
}
