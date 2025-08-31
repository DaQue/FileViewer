const openBtn = document.getElementById('open');
const fitChk = document.getElementById('fit');
const zmMinus = document.getElementById('zm-');
const zmPlus = document.getElementById('zm+');
const z100 = document.getElementById('z100');
const textEl = document.getElementById('text');
const imgEl = document.getElementById('image');
const statusEl = document.getElementById('status');
const findInput = document.getElementById('find');
const matchesEl = document.getElementById('matches');

let currentPath = null;
let imageZoom = 1.0;

openBtn.addEventListener('click', async () => {
  const info = await window.api.open();
  if (!info) return;
  if (info.kind === 'error') {
    alert(info.error);
    return;
  }
  currentPath = info.path;
  if (info.kind === 'text') {
    imgEl.classList.add('hidden');
    textEl.classList.remove('hidden');
    textEl.textContent = info.text;
    statusEl.textContent = `${info.path} — ${info.text.split(/\n/).length} lines`;
    updateMatches();
  } else if (info.kind === 'image') {
    textEl.classList.add('hidden');
    imgEl.classList.remove('hidden');
    imageZoom = 1.0;
    fitChk.checked = false;
    imgEl.style.transform = `scale(${imageZoom})`;
    imgEl.src = info.path;
    statusEl.textContent = `${info.path}`;
  }
});

function updateImageTransform() {
  if (fitChk.checked && imgEl.naturalWidth && imgEl.naturalHeight) {
    const cw = document.getElementById('content').clientWidth;
    const ch = document.getElementById('content').clientHeight;
    const sx = cw / imgEl.naturalWidth;
    const sy = ch / imgEl.naturalHeight;
    const s = Math.max(0.1, Math.min(6.0, Math.min(sx, sy)));
    imgEl.style.transform = `scale(${s})`;
  } else {
    imgEl.style.transform = `scale(${imageZoom})`;
  }
}

fitChk.addEventListener('change', () => updateImageTransform());
zmMinus.addEventListener('click', () => { fitChk.checked = false; imageZoom = Math.max(0.1, imageZoom/1.1); updateImageTransform(); });
zmPlus.addEventListener('click', () => { fitChk.checked = false; imageZoom = Math.min(6.0, imageZoom*1.1); updateImageTransform(); });
z100.addEventListener('click', () => { fitChk.checked = false; imageZoom = 1.0; updateImageTransform(); });

document.getElementById('content').addEventListener('wheel', (e) => {
  if (imgEl.classList.contains('hidden')) return;
  if (!imgEl.matches(':hover')) return;
  e.preventDefault();
  fitChk.checked = false;
  imageZoom = Math.max(0.1, Math.min(6.0, imageZoom * (e.deltaY < 0 ? 1.1 : 1/1.1)));
  updateImageTransform();
}, { passive: false });

findInput.addEventListener('input', updateMatches);

function updateMatches() {
  matchesEl.textContent = '';
  const q = findInput.value;
  if (!q || textEl.classList.contains('hidden')) return;
  const text = textEl.textContent;
  // Naive highlighting: rebuild content with spans (safe for moderate files)
  const chunks = [];
  let rest = text;
  let count = 0;
  while (rest.length) {
    const idx = rest.toLowerCase().indexOf(q.toLowerCase());
    if (idx < 0) { chunks.push(escapeHtml(rest)); break; }
    chunks.push(escapeHtml(rest.slice(0, idx)));
    const match = rest.slice(idx, idx + q.length);
    chunks.push(`<span class="match">${escapeHtml(match)}</span>`);
    rest = rest.slice(idx + q.length);
    count++;
  }
  textEl.innerHTML = chunks.join('');
  matchesEl.textContent = `${count} match(es)`;
}

function escapeHtml(s) {
  return s.replace(/[&<>]/g, (c) => ({'&':'&amp;','<':'&lt;','>':'&gt;'}[c]));
}

window.addEventListener('resize', updateImageTransform);

