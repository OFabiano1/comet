// Comet — renderer. Talks only through window.comet (preload bridge).
const $ = (id) => document.getElementById(id);

const urlInput = $('url');
const actionButtons = ['btn-mp4', 'btn-mp3', 'btn-gif'].map((id) => $(id));
const stopButton = $('btn-stop');
const progressWrap = $('progress-wrap');
const progressBar = $('progress');
const statusEl = $('status');
const folderButton = $('btn-folder');
const folderPath = $('folder-path');
const updateButton = $('btn-ytdlp-update');
const versionsEl = $('versions');

function setStatus(text) {
  statusEl.textContent = text;
}

function setBusy(busy) {
  for (const b of actionButtons) b.disabled = busy || urlInput.value.trim().length === 0;
  folderButton.disabled = busy;
  updateButton.disabled = busy;
  stopButton.hidden = !busy;
  progressWrap.hidden = !busy;
  if (!busy) {
    progressBar.value = 0;
    updateButtonsEnabled();
  } else {
    delete progressWrap.dataset.indeterminate;
  }
}

function updateButtonsEnabled() {
  const hasText = urlInput.value.trim().length > 0;
  for (const b of actionButtons) b.disabled = !hasText;
}

async function startDownload(kind) {
  const url = urlInput.value.trim();
  if (!url) return;
  setBusy(true);
  setStatus('Baixando...');
  let result;
  try {
    result = await window.comet.download(url, kind, {});
  } catch (err) {
    setStatus('Erro inesperado, tenta de novo.');
    setBusy(false);
    return;
  }
  if (result.ok) {
    setStatus(`Pronto: ${result.file || 'arquivo baixado'}`);
  } else if (result.error) {
    setStatus(result.error);
  }
  setBusy(false);
}

actionButtons[0].addEventListener('click', () => startDownload('mp4'));
actionButtons[1].addEventListener('click', () => startDownload('mp3'));
actionButtons[2].addEventListener('click', () => startDownload('gif'));

stopButton.addEventListener('click', () => {
  setStatus('Parando...');
  window.comet.cancel();
});

urlInput.addEventListener('input', updateButtonsEnabled);

folderButton.addEventListener('click', async () => {
  const picked = await window.comet.setFolder();
  if (picked.ok) {
    const current = await window.comet.getFolder();
    if (current.ok) folderPath.textContent = current.path;
  } else if (picked.error) {
    setStatus(picked.error);
  }
});

updateButton.addEventListener('click', async () => {
  setBusy(true);
  setStatus('Atualizando o yt-dlp...');
  let result;
  try {
    result = await window.comet.updateytDlp();
  } catch {
    setBusy(false);
    setStatus('Não foi possível atualizar agora.');
    return;
  }
  setBusy(false);
  if (result.ok) {
    setStatus(result.text || 'yt-dlp atualizado.');
  } else {
    setStatus(result.error || 'Não foi possível atualizar agora.');
  }
});

window.comet.onEvent((payload) => {
  switch (payload.type) {
    case 'progress': {
      const pct = payload.percent;
      if (pct == null) {
        progressWrap.dataset.indeterminate = '';
        setStatus('Processando...');
      } else {
        progressBar.value = pct;
        setStatus(`Baixando... ${pct}%`);
      }
      break;
    }
    case 'status':
      setStatus(payload.text);
      break;
    case 'error':
      setStatus(payload.text);
      setBusy(false);
      break;
    case 'done':
      setStatus(payload.text);
      setBusy(false);
      break;
    default:
      break;
  }
});

(async () => {
  const versions = await window.comet.getVersions();
  versionsEl.textContent = `Comet ${versions.app} · Electron ${versions.electron}`;
  const current = await window.comet.getFolder();
  if (current.ok) folderPath.textContent = current.path;
})();
