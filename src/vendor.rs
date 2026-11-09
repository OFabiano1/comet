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

/// Sufixo único por processo e por chamada: tarefas concorrentes nunca
/// escrevem no mesmo arquivo temporário.
fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

async fn fetch_to_file(app: &Arc<Mutex<App>>, url: &str, dest: &Path, label: &str) -> Result<()> {
    // Temp único por processo: duas instâncias não escrevem no mesmo arquivo.
    let temp = dest.with_extension(format!("tmp-{}", unique_suffix()));
    let resp = reqwest::get(url).await?.error_for_body()?;
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(&temp).await?;
    let mut stream = resp.bytes_stream();
    let mut got: u64 = 0;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.try_next().await.map_err(|e| anyhow::anyhow!(e))? {
        file.write_all(&chunk).await?;
        got += chunk.len() as u64;
        let msg = match total {
            Some(t) => format!("{label} · {:.0}%", got as f64 / t as f64 * 100.0),
            None => format!("{label} · {:.1} MB", got as f64 / 1e6),
        };
        set_status(app, msg).await;
    }
    file.flush().await?;
    drop(file);
    // Destino já existe (outra instância resolveu antes): mantém o existente.
    if dest.exists() {
        let _ = tokio::fs::remove_file(&temp).await;
    } else if let Err(e) = tokio::fs::rename(&temp, dest).await {
        let _ = tokio::fs::remove_file(&temp).await;
        if !dest.exists() {
            return Err(anyhow::anyhow!(e));
        }
    }
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
    let staging = dest_dir.join(format!(".staging-{}", unique_suffix()));
    std::fs::create_dir_all(&staging)?;
    let mut found_ffmpeg = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_ascii_lowercase();
        let base = match name.rsplit('/').next() {
            Some("ffmpeg.exe") => {
                found_ffmpeg = true;
                "ffmpeg.exe"
            }
            Some("ffprobe.exe") => "ffprobe.exe",
            _ => continue,
        };
        let mut out = std::fs::File::create(staging.join(base))?;
        std::io::copy(&mut entry, &mut out)?;
    }
    // Binário já instalado nunca é sobrescrito: evita corromper outra instância.
    for base in ["ffmpeg.exe", "ffprobe.exe"] {
        let staged = staging.join(base);
        let dest = dest_dir.join(base);
        if staged.is_file() && !dest.exists() {
            let _ = std::fs::rename(&staged, &dest);
        }
    }
    let _ = std::fs::remove_dir_all(&staging);
    if !found_ffmpeg && !dest_dir.join("ffmpeg.exe").is_file() {
        anyhow::bail!("o zip baixado não tem ffmpeg");
    }
    Ok(())
}

/// Nome estável para os testes e para a instalação.
pub fn install_ffmpeg_zip(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    extract_exes(zip_path, dest_dir)
}

pub async fn ensure_binaries(app: &Arc<Mutex<App>>) -> Result<(PathBuf, PathBuf)> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf));
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
        set_status(app, "baixando yt-dlp".to_string()).await;
        fetch_to_file(app, YTDLP_URL, &ytdlp, "baixando yt-dlp").await?;
    }
    if !ffmpeg.is_file() {
        let zip_path = dir.join("ffmpeg.zip");
        let mut last_err = None;
        for url in FFMPEG_URLS {
            set_status(app, "baixando ffmpeg".to_string()).await;
            match fetch_to_file(app, url, &zip_path, "baixando ffmpeg").await {
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
        set_status(app, "extraindo ffmpeg".to_string()).await;
        let z = zip_path.clone();
        let d = dir.clone();
        tokio::task::spawn_blocking(move || install_ffmpeg_zip(&z, &d))
            .await
            .map_err(|e| anyhow::anyhow!(e))??;
        let _ = tokio::fs::remove_file(&zip_path).await;
    }
    if !ffprobe.is_file() {
        anyhow::bail!("ffprobe não foi extraído");
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

    #[test]
    fn install_ffmpeg_does_not_overwrite_existing_binary() {
        let dir = std::env::temp_dir().join("comet-install-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("ffmpeg.zip");
        make_fake_ffmpeg_zip(&zip_path, b"from-zip");
        let dest = dir.join("bin");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("ffmpeg.exe"), b"already-installed").unwrap();

        install_ffmpeg_zip(&zip_path, &dest).unwrap();
        assert_eq!(
            std::fs::read(dest.join("ffmpeg.exe")).unwrap(),
            b"already-installed",
            "binario instalado foi sobrescrito"
        );
        assert!(dest.join("ffprobe.exe").is_file());
        let leftovers: Vec<_> = std::fs::read_dir(&dest)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".staging"))
            .collect();
        assert!(leftovers.is_empty(), "staging nao limpo: {:?}", leftovers);
    }

    fn make_fake_ffmpeg_zip(path: &Path, content: &[u8]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("bin/ffmpeg.exe", opts).unwrap();
        std::io::Write::write_all(&mut zip, content).unwrap();
        zip.start_file("bin/ffprobe.exe", opts).unwrap();
        std::io::Write::write_all(&mut zip, content).unwrap();
        zip.finish().unwrap();
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
        crate::downloader::download(&app, 0, &yt, &ff)
            .await
            .unwrap();
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
