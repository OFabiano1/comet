use crate::app::{App, HistEntry, Item, Kind, Phase, Progress, VideoInfo};
use crate::sites::Site;
use anyhow::Result;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Números crus: o Comet calcula %, quanto falta e formata do jeito dele.
const PROGRESS_TEMPLATE: &str = "download:[PROG]%(progress.downloaded_bytes)s|%(progress.total_bytes,progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.vcodec)s";

pub fn parse_title(line: &str) -> Option<String> {
    line.strip_prefix("TITLE:")
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// yt-dlp escreve "NA" quando não sabe o valor.
fn known(field: &str) -> Option<&str> {
    let f = field.trim();
    (!f.is_empty() && f != "NA" && f != "None").then_some(f)
}

fn number(field: &str) -> Option<f64> {
    known(field)?
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.0)
}

/// `INFO:canal|segundos`
pub fn parse_info(line: &str) -> Option<VideoInfo> {
    let (channel, duration) = line.strip_prefix("INFO:")?.split_once('|')?;
    Some(VideoInfo {
        channel: known(channel).unwrap_or_default().to_string(),
        duration_secs: number(duration).map(|d| d.round() as u64),
    })
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

/// `[PROG]baixado|total|velocidade|eta|vcodec` (ver PROGRESS_TEMPLATE).
pub fn parse_progress(line: &str) -> Option<Progress> {
    let mut parts = line.strip_prefix("[PROG]")?.split('|');
    let downloaded = number(parts.next()?)? as u64;
    let total = parts.next().and_then(number).map(|t| t as u64);
    let speed = parts.next().and_then(number);
    let eta_secs = parts.next().and_then(number).map(|e| e as u64);
    // Parte só de áudio vem com vcodec "none".
    let audio = parts.next().map(str::trim) == Some("none");
    Some(Progress {
        downloaded,
        total,
        speed,
        eta_secs,
        audio,
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

/// `FILE:<caminho final>` — prefixo explícito; caminho com espaço é o caso comum.
pub fn parse_saved(line: &str) -> Option<PathBuf> {
    line.strip_prefix("FILE:")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
}

/// Título de tweet/TikTok costuma ser só "Fulano - https://t.co/…": vira nome ilegível
/// e repetido. Tira link, sobra de separador e, se nada sobrar, chama de "vídeo".
pub fn title_cleanup_args() -> Vec<String> {
    [
        (r"https?://\S+", ""),
        (r"^[\s\-–—:|]+|[\s\-–—:|]+$", ""),
        (r"^$", "vídeo"),
    ]
    .iter()
    .flat_map(|(re, to)| ["--replace-in-metadata", "title", re, to])
    .map(str::to_string)
    .collect()
}

/// Nome do arquivo = título, com "Comet - " na frente.
/// shortcut: título repetido (tweet/TikTok) faz o yt-dlp pular o download, sem sufixo numérico; upgrade se virar problema.
pub fn output_template(site: Site) -> &'static str {
    match site {
        Site::YouTube => "Comet - %(title).80B.%(ext)s",
        Site::Twitter | Site::TikTok => "Comet - %(title).60B.%(ext)s",
    }
}

/// Arquivo de histórico ao lado do bin (%LOCALAPPDATA%\comet\historico.txt).
pub fn history_path() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|l| PathBuf::from(l).join("comet").join("historico.txt"))
        .unwrap_or_else(|_| PathBuf::from("comet-historico.txt"))
}

/// Linha do histórico: "gif · twitter · título".
pub fn history_label(item: &Item) -> String {
    let kind = match item.kind {
        Kind::Video => "vídeo",
        Kind::Gif => "gif",
    };
    format!("{kind} · {} · {}", item.site.label(), item.title).replace(['\t', '\n'], " ")
}

/// Anota um download concluído: "label\tcaminho\turl". Falha de escrita não derruba a fila.
pub fn record_history(item: &Item) -> HistEntry {
    let label = history_label(item);
    let path = item.saved.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
    let file = history_path();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
        let _ = writeln!(f, "{label}\t{path}\t{}", item.url);
    }
    HistEntry {
        label,
        url: item.url.clone(),
    }
}

/// Uma linha do arquivo. Linha antiga, sem url, fica com url vazia.
pub fn parse_history_line(line: &str) -> HistEntry {
    let mut parts = line.split('\t');
    HistEntry {
        label: parts.next().unwrap_or_default().to_string(),
        url: parts.nth(1).unwrap_or_default().to_string(),
    }
}

/// Histórico salvo, mais novo primeiro. Sem arquivo = histórico vazio.
pub fn load_history() -> Vec<HistEntry> {
    let text = std::fs::read_to_string(history_path()).unwrap_or_default();
    text.lines().rev().map(parse_history_line).collect()
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
    ffmpeg: &Path,
) -> Result<()> {
    let (url, site, out_dir) = {
        let mut a = app.lock().await;
        let item = &mut a.items[idx];
        item.phase = Phase::Downloading;
        item.progress = Progress::default();
        (item.url.clone(), item.site, downloads_dir())
    };
    tokio::fs::create_dir_all(&out_dir).await?;

    let mut args: Vec<String> = vec![
        // --print liga o modo silencioso, que esconde o progresso; --progress traz de volta.
        "--progress".to_string(),
        "--newline".to_string(),
        "--no-playlist".to_string(),
        "--merge-output-format".to_string(),
        "mp4".to_string(),
        "--progress-template".to_string(),
        PROGRESS_TEMPLATE.to_string(),
        "--print".to_string(),
        "after_move:FILE:%(filepath)s".to_string(),
        "--print".to_string(),
        "TITLE:%(title)s".to_string(),
        "--print".to_string(),
        "INFO:%(uploader,channel)s|%(duration)s".to_string(),
        "-o".to_string(),
        output_template(site).to_string(),
    ];
    args.extend(title_cleanup_args());
    args.push("--ffmpeg-location".to_string());
    args.push(ffmpeg.display().to_string());
    // YouTube não precisa de login; ler cookies do Chrome aberto só faria ele falhar
    if site != Site::YouTube {
        args.extend(browser_cookie_args());
    }
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
                        app2.lock().await.items[idx].progress = p;
                    } else if let Some(title) = parse_title(l) {
                        app2.lock().await.items[idx].title = title;
                    } else if let Some(info) = parse_info(l) {
                        app2.lock().await.items[idx].info = info;
                    } else if let Some(path) = parse_saved(l) {
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
            a.items[idx].saved = final_path.clone();
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
            to_gif(app, idx, ffmpeg, &mp4).await?;
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

/// Duração máxima do GIF em segundos; a paleta usa o mesmo trecho.
pub const GIF_MAX_SECS: &str = "30";

pub fn palette_args(input: &Path, png: &Path) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-t".to_string(),
        GIF_MAX_SECS.to_string(),
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
        "-t".to_string(),
        GIF_MAX_SECS.to_string(),
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
        Ok(()) => {
            item.phase = Phase::Done;
            item.saved = Some(gif);
        }
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
        // Linhas reais do yt-dlp (vídeo av01 e depois a parte de áudio).
        let p = parse_progress("[PROG]3072|223779|1948420.064721004|0|av01.0.00M.08").unwrap();
        assert_eq!(p.downloaded, 3072);
        assert_eq!(p.total, Some(223779));
        assert_eq!(p.speed, Some(1948420.064721004));
        assert_eq!(p.eta_secs, Some(0));
        assert!(!p.audio);
        assert!((p.pct().unwrap() - 1.37).abs() < 0.01);
        assert_eq!(p.remaining(), Some(220707));

        let p = parse_progress("[PROG]1024|NA|NA|NA|none").unwrap();
        assert_eq!(p.total, None);
        assert_eq!(p.pct(), None);
        assert!(p.audio);

        assert!(parse_progress("[PROG]NA|NA|NA|NA|NA").is_none());
        assert!(parse_progress("garbage").is_none());
        assert!(parse_progress("[download] Destination: x.mp4").is_none());
    }

    #[test]
    fn parses_info_line() {
        assert_eq!(
            parse_info("INFO:jawed|19"),
            Some(VideoInfo {
                channel: "jawed".into(),
                duration_secs: Some(19)
            })
        );
        // Canal com barra não pode virar caminho de arquivo.
        assert_eq!(parse_info("INFO:AC/DC|NA").unwrap().channel, "AC/DC");
        assert_eq!(parse_info("INFO:NA|12.5").unwrap().channel, "");
        assert_eq!(parse_info("TITLE:x"), None);
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
    fn parses_saved_file_line_with_spaces() {
        // Antes, linha com espaço era descartada: GIF falhava em quase todo título.
        assert_eq!(
            parse_saved("FILE:C:\\Users\\F\\Downloads\\Comet\\Me at the zoo.mp4"),
            Some(PathBuf::from(
                "C:\\Users\\F\\Downloads\\Comet\\Me at the zoo.mp4"
            ))
        );
        assert_eq!(parse_saved("C:\\x\\video.mp4"), None);
        assert_eq!(parse_saved("[download] 100% of 2MiB"), None);
        assert_eq!(parse_saved("FILE:"), None);
    }

    #[test]
    fn output_names_start_with_comet_and_title() {
        assert_eq!(output_template(Site::YouTube), "Comet - %(title).80B.%(ext)s");
        assert_eq!(output_template(Site::Twitter), "Comet - %(title).60B.%(ext)s");
        assert_eq!(output_template(Site::TikTok), "Comet - %(title).60B.%(ext)s");
        let args = title_cleanup_args();
        assert_eq!(args.len(), 12);
        assert_eq!(
            args[..3],
            ["--replace-in-metadata", "title", r"https?://\S+"]
        );
    }

    #[test]
    fn history_label_has_kind_site_and_title() {
        let item = Item {
            url: String::new(),
            title: "Layne\tcom quebra".into(),
            site: Site::Twitter,
            kind: Kind::Gif,
            phase: Phase::Done,
            info: VideoInfo::default(),
            progress: Progress::default(),
            error: None,
            saved: None,
        };
        assert_eq!(history_label(&item), "gif · twitter · Layne com quebra");
    }

    #[test]
    fn history_line_keeps_url_for_redownload() {
        let e = parse_history_line("gif · twitter · Layne\tC:\\x.gif\thttps://x.com/u/status/1");
        assert_eq!(e.label, "gif · twitter · Layne");
        assert_eq!(e.url, "https://x.com/u/status/1");
        assert_eq!(parse_history_line("linha antiga").url, "");
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
                "-t".to_string(),
                GIF_MAX_SECS.to_string(),
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
                "-t".to_string(),
                GIF_MAX_SECS.to_string(),
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
            info: VideoInfo::default(),
            progress: Progress::default(),
            error: None,
            saved: None,
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
