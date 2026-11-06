// Comet — renderer. Placeholder until Task 8 wires the real UI behavior.
const $ = (id) => document.getElementById(id);

const urlInput = $('url');
const buttons = ['btn-mp4', 'btn-mp3', 'btn-gif'].map($);
const status = $('status');

urlInput.addEventListener('input', () => {
  const hasText = urlInput.value.trim().length > 0;
  for (const b of buttons) b.disabled = !hasText;
});

for (const b of buttons) {
  b.addEventListener('click', () => {
    status.textContent = 'Nem tudo instalado ainda.';
  });
}
