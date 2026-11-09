use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use futures_util::TryStreamExt;

use crate::app::App;

pub fn bin_dir() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|l| PathBuf::from(l).join("comet").join("bin"))
        .unwrap_or_else(|_| PathBuf::from("comet-bin"))
}

pub fn candidate_paths(exe_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(dir) = exe_dir {
        v.push(dir.join("vendor"));
        v.push(dir.to_path_buf());
    }
    v.push(bin_dir());
    v
}

const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const FFMPEG_URLS: [&str; 2] = [
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
    "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip",
];

async fn set_status(app: &Arc<Mutex<App>>, msg: String) {
    app.lock().await.status = msg;
}

async fn fetch_to_file(app: &Arc<Mutex<App>>, url: &str, dest: &Path, label: &str) -> Result<()> {
    let resp = reqwest::get(url).await?.error_for_body()?;
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = resp.bytes_stream();
    let mut got: u64 = 0;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.try_next().await.map_err(|e| anyhow::anyhow!(e))? {        file.write_all(&chunk).await?;
        got += chunk.len() as u64;
        let msg = match total {
            Some(t) => format!("{label} · {:.0}%", got as f64 / t as f64 * 100.0),
            None => format!("{label} · {:.1} MB", got as f64 / 1e6),
        };
        set_status(app, msg).await;
    }
    file.flush().await?;
    Ok(())
}

trait ErrorForBody {
    fn error_for_body(self) -> Result<reqwest::Response>;
}

impl ErrorForBody for reqwest::Response {
    fn error_for_body(self) -> Result<reqwest::Response> {
        self.error_for_status().map_err(|e| anyhow::anyhow!(e))
    }
}

fn extract_exes(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_ascii_lowercase();
        let base = match name.rsplit('/').next() {
            Some("ffmpeg.exe") => "ffmpeg.exe",
            Some("ffprobe.exe") => "ffprobe.exe",
            _ => continue,
        };
        let mut out = std::fs::File::create(dest_dir.join(base))?;
        std::io::copy(&mut entry, &mut out)?;
    }
    if !dest_dir.join("ffmpeg.exe").is_file() {
        anyhow::bail!("FFMPEG NÃO ENCONTRADO NO ZIP");
    }
    Ok(())
}

pub async fn ensure_binaries(app: &Arc<Mutex<App>>) -> Result<(PathBuf, PathBuf)> {
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    for dir in candidate_paths(exe_dir.as_deref()) {
        let ytdlp = dir.join("yt-dlp.exe");
        let ffmpeg = dir.join("ffmpeg.exe");
        if ytdlp.is_file() && ffmpeg.is_file() {
            return Ok((ytdlp, ffmpeg));
        }
    }
    let dir = bin_dir();
    tokio::fs::create_dir_all(&dir).await?;
    let ytdlp = dir.join("yt-dlp.exe");
    let ffmpeg = dir.join("ffmpeg.exe");
    let ffprobe = dir.join("ffprobe.exe");

    if !ytdlp.is_file() {
        set_status(app, "BAIXANDO YT-DLP".to_string()).await;
        fetch_to_file(app, YTDLP_URL, &ytdlp, "BAIXANDO YT-DLP").await?;
    }
    if !ffmpeg.is_file() {
        let zip_path = dir.join("ffmpeg.zip");
        let mut last_err = None;
        for url in FFMPEG_URLS {
            set_status(app, "BAIXANDO FFMPEG".to_string()).await;
            match fetch_to_file(app, url, &zip_path, "BAIXANDO FFMPEG").await {
                Ok(()) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    let _ = tokio::fs::remove_file(&zip_path).await;
                }
            }
        }
        if let Some(e) = last_err {
            return Err(e);
        }
        set_status(app, "EXTRAINDO FFMPEG".to_string()).await;
        let z = zip_path.clone();
        let d = dir.clone();
        tokio::task::spawn_blocking(move || extract_exes(&z, &d))
            .await
            .map_err(|e| anyhow::anyhow!(e))??;
        let _ = tokio::fs::remove_file(&zip_path).await;
    }
    if !ffprobe.is_file() {
        anyhow::bail!("FFPROBE NÃO EXTRAÍDO");
    }
    Ok((ytdlp, ffmpeg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_priority_exe_vendor_first() {
        let exe = Path::new("C:\\app");
        let v = candidate_paths(Some(exe));
        assert_eq!(v[0], PathBuf::from("C:\\app\\vendor"));
        assert_eq!(v[1], PathBuf::from("C:\\app"));
        assert_eq!(v[2], bin_dir());
        assert_eq!(candidate_paths(None).len(), 1);
    }

    #[test]
    fn bin_dir_uses_localappdata_comet() {
        if let Ok(l) = std::env::var("LOCALAPPDATA") {
            assert_eq!(bin_dir(), PathBuf::from(l).join("comet").join("bin"));
        }
    }

    #[tokio::test]
    #[ignore = "download real de ~90 MB; rodar com `cargo test --ignored`"]
    async fn fetches_missing_vendor_binaries_for_real() {
        let app = Arc::new(Mutex::new(App::default()));
        let (yt, ff) = ensure_binaries(&app).await.unwrap();
        assert!(yt.is_file());
        assert!(ff.is_file());
    }

    #[tokio::test]
    #[ignore = "download real de rede; rodar com `cargo test --ignored`"]
    async fn real_youtube_download_end_to_end() {
        let app = Arc::new(Mutex::new(App::default()));
        app.lock().await.items.push(crate::app::Item {
            url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".into(),
            title: "…".into(),
            site: crate::sites::Site::YouTube,
            kind: crate::app::Kind::Video,
            phase: crate::app::Phase::Queued,
            pct: 0.0,
            size_str: String::new(),
            got_str: String::new(),
            error: None,
        });
        let (yt, ff) = ensure_binaries(&app).await.unwrap();
        crate::downloader::download(&app, 0, &yt, &ff).await.unwrap();
        let a = app.lock().await;
        assert_eq!(
            a.items[0].phase,
            crate::app::Phase::Done,
            "erro: {:?}",
            a.items[0].error
        );
        assert_eq!(a.completed, 1);
        assert!(
            a.items[0].title.to_lowercase().contains("zoo"),
            "título não preenchido: {:?}",
            a.items[0].title
        );
    }
}
