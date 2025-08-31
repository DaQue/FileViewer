const openBtn = document.getElementById(''open'');
const recentSel = document.getElementById(''recent'');
const fitChk = document.getElementById(''fit'');
const wrapChk = document.getElementById(''wrap'');
const darkChk = document.getElementById(''dark'');
const zmMinus = document.getElementById(''zm-'');
const zmPlus = document.getElementById(''zm+'');
const z100 = document.getElementById(''z100'');
const textEl = document.getElementById(''text'');
const imgEl = document.getElementById(''image'');
const statusEl = document.getElementById(''status'');
const findInput = document.getElementById(''find'');
const matchesEl = document.getElementById(''matches'');
const copyBtn = document.getElementById(''copy'');
const revealBtn = document.getElementById(''reveal'');
const prevBtn = document.getElementById(''prev'');
const nextBtn = document.getElementById(''next'');

let currentPath = null;
let currentExt = '';
let imageZoom = 1.0;
const MAX_IMAGE_TEXTURE_BYTES = 128 * 1024 * 1024; // ~128MB

openBtn.addEventListener('click', async () => {
  const info = await window.api.open();
  if (!info) return;
  handleOpenInfo(info);
});

async function handleOpenInfo(info) {
  if (info.kind === ''error'') { alert(info.error); return; }
  currentPath = info.path;
  currentExt = (currentPath.split(''.'').pop()||'''').toLowerCase();
  if (info.kind === ''text'') {
    imgEl.classList.add(''hidden'');
    textEl.classList.remove(''hidden'');
    statusEl.textContent = `${info.path} - ${info.text.split(/\n/).length} lines`;
    addRecent(info.path);
    renderText(info.text, currentExt, findInput.value);
    // re-apply wrap on new content
    const st = getSettings(); applyWrap(st.wrap !== false);
  } else if (info.kind === ''image'') {
    textEl.classList.add(''hidden'');
    imgEl.classList.remove(''hidden'');
    imageZoom = 1.0;
    fitChk.checked = false;
    imgEl.style.transform = `scale(${imageZoom})`;
    imgEl.src = info.path;
    imgEl.onload = () => {
      const est = (imgEl.naturalWidth * imgEl.naturalHeight * 4);
      if (est > MAX_IMAGE_TEXTURE_BYTES) {
        alert(`Image too large: ${imgEl.naturalWidth}x${imgEl.naturalHeight} (~${(est/1024/1024).toFixed(1)} MB RGBA). Limit ~${(MAX_IMAGE_TEXTURE_BYTES/1024/1024)} MB`);
        imgEl.src = '';
        textEl.classList.add('hidden');
        imgEl.classList.add('hidden');
        statusEl.textContent = 'Image rejected due to size';
        return;
      }
      updateImageStatus();
    };
    addRecent(info.path);
  }
}

function updateImageTransform() {
  if (fitChk.checked && imgEl.naturalWidth && imgEl.naturalHeight) {
    const cw = document.getElementById(''content'').clientWidth;
    const ch = document.getElementById(''content'').clientHeight;
    const sx = cw / imgEl.naturalWidth;
    const sy = ch / imgEl.naturalHeight;
    const s = Math.max(0.1, Math.min(6.0, Math.min(sx, sy)));
    imgEl.style.transform = `scale(${s})`;
  } else {
    imgEl.style.transform = `scale(${imageZoom})`;
  }
  updateImageStatus();
  try { const s = getSettings(); s.imageZoom = imageZoom; setSettings(s); } catch {}
}

fitChk.addEventListener('change', () => updateImageTransform());
zmMinus.addEventListener('click', () => { fitChk.checked = false; imageZoom = Math.max(0.1, imageZoom/1.1); updateImageTransform(); });
zmPlus.addEventListener('click', () => { fitChk.checked = false; imageZoom = Math.min(6.0, imageZoom*1.1); updateImageTransform(); });
z100.addEventListener('click', () => { fitChk.checked = false; imageZoom = 1.0; updateImageTransform(); });

document.getElementById(''content'').addEventListener('wheel', (e) => {
  if (imgEl.classList.contains(''hidden'')) return;
  if (!imgEl.matches('':hover'')) return;
  e.preventDefault();
  fitChk.checked = false;
  imageZoom = Math.max(0.1, Math.min(6.0, imageZoom * (e.deltaY < 0 ? 1.1 : 1/1.1)));
  updateImageTransform();
}, { passive: false });

findInput.addEventListener('input', () => {
  if (!textEl.classList.contains(''hidden'')) {
    renderText(textEl.textContent, currentExt, findInput.value);
  }
});

// Find navigation
let findIndex = 0;
if (nextBtn) nextBtn.addEventListener('click', () => stepFind(1));
if (prevBtn) prevBtn.addEventListener('click', () => stepFind(-1));
findInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') stepFind(1); });

function stepFind(dir) {
  if (textEl.classList.contains('hidden')) return;
  const matches = Array.from(textEl.querySelectorAll('.match'));
  if (!matches.length) return;
  findIndex = (findIndex + dir + matches.length) % matches.length;
  matches.forEach(m => m.classList.remove('current'));
  const el = matches[findIndex]; el.classList.add('current'); el.scrollIntoView({ block: 'center' });
}

function renderText(text, ext, query) {
  const lines = text.split("\n");
  let html = '';
  let count = 0;
  let depth = 0;
  const showLN = (getSettings().ln !== false);
  let ln = 1;
  for (const line of lines) {
    const code = highlightLine(line, ext, query, () => depth, (d) => { depth = d; });
    html += `<span class="gutter">${showLN ? ln : ''}</span>${code}` + '\n';
    ln++;
  }
  textEl.innerHTML = html;
  if (query) {
    const lc = text.toLowerCase();
    const ql = query.toLowerCase();
    let pos = 0; count = 0;
    while (true) { const i = lc.indexOf(ql, pos); if (i < 0) break; count++; pos = i + ql.length; }
  }
  matchesEl.textContent = count ? `${count} match(es)` : '';
}

function highlightLine(line, ext, query, getDepth, setDepth) {
  const base = (s) => escapeHtml(s);
  const kw = (s) => `<span style="color:#61afef">${escapeHtml(s)}</span>`;
  const str = (s) => `<span style="color:#98c379">${escapeHtml(s)}</span>`;
  const com = (s) => `<span style="color:gray">${escapeHtml(s)}</span>`;
  const num = (s) => `<span style="color:#d19a66">${escapeHtml(s)}</span>`;
  const boolc = (s) => `<span style="color:#c678dd">${escapeHtml(s)}</span>`;
  const palette = ['#98c379','#e06c75','#61afef','#e5c07b','#56b6c2'];
  const bracket = (s, open) => {
    let d = getDepth();
    if (open) {
      const idx = Math.max(0, d) % palette.length;
      setDepth(d + 1);
      return `<span style="color:${palette[idx]}">${escapeHtml(s)}</span>`;
    } else {
      setDepth(Math.max(-1000, getDepth() - 1));
      const idx = Math.max(0, getDepth()) % palette.length;
      return `<span style="color:${palette[idx]}">${escapeHtml(s)}</span>`;
    }
  };
  // Comments first
  let cidx = -1;
  if (ext === 'rs') cidx = line.indexOf('//');
  if (ext === 'py' || ext === 'toml') cidx = line.indexOf('#');
  if (cidx >= 0) {
    return highlightLine(line.slice(0,cidx), ext, query, getDepth, setDepth) + com(line.slice(cidx));
  }
  // Tokenize
  let out = '';
  let i = 0; let buf = '';
  const pushWord = () => {
    if (!buf) return;
    const lc = buf.toLowerCase();
    if (ext === 'py' && PY_KW.has(buf)) out += kw(buf);
    else if (ext === 'rs' && RS_KW.has(buf)) out += kw(buf);
    else if (lc === 'true' || lc === 'false' || lc === 'null' || lc === 'none') out += boolc(buf);
    else if (/^\d+$/.test(buf)) out += num(buf);
    else out += base(buf);
    buf = '';
  };
  while (i < line.length) {
    const ch = line[i];
    if (ch === '"') {
      pushWord();
      let j = i+1; while (j < line.length && line[j] !== '"') j++;
      out += str(line.slice(i, Math.min(line.length, j+1)));
      i = j+1; continue;
    }
    if (/\w/.test(ch)) { buf += ch; i++; continue; }
    pushWord();
    if (ch === '(' || ch === '[' || ch === '}') { out += bracket(ch, true); i++; continue; }
    if (ch === ')' || ch === ']' || ch === '{') { out += bracket(ch, false); i++; continue; }
    out += base(ch);
    i++;
  }
  pushWord();
  return out;
}

const RS_KW = new Set(['as','async','await','break','const','continue','crate','dyn','else','enum','extern','false','fn','for','if','impl','in','let','loop','match','mod','move','mut','pub','ref','return','self','Self','static','struct','super','trait','true','type','unsafe','use','where','while','union','box','try','yield','macro','macro_rules']);
const PY_KW = new Set(['False','None','True','and','as','assert','async','await','break','class','continue','def','del','elif','else','except','finally','for','from','global','if','import','in','is','lambda','nonlocal','not','or','pass','raise','return','try','while','with','yield','match','case']);

function escapeHtml(s) { return s.replace(/[&<>]/g, (c) => ({'&':'&amp;','<':'&lt;','>':'&gt;'}[c])); }

// Recents in localStorage
function getRecents() { try { return JSON.parse(localStorage.getItem('recents')||'[]'); } catch { return [] } }
function setRecents(list) { localStorage.setItem('recents', JSON.stringify(list.slice(-10))); refreshRecents(); }
function addRecent(p) { const rec = getRecents().filter(x => x !== p); rec.push(p); setRecents(rec); }
function refreshRecents() {
  const rec = getRecents();
  recentSel.innerHTML = '';
  if (!rec.length) { const opt = document.createElement('option'); opt.value=''; opt.textContent='(empty)'; recentSel.appendChild(opt); }
  else {
    const opt0 = document.createElement('option'); opt0.value=''; opt0.textContent='Recent…'; recentSel.appendChild(opt0);
    for (let i=rec.length-1;i>=0;i--) { const opt = document.createElement('option'); opt.value = rec[i]; opt.textContent = rec[i]; recentSel.appendChild(opt); }
  }
}
recentSel.addEventListener('change', async () => {
  const p = recentSel.value; if (!p) return;
  const info = await window.api.openPath(p);
  if (!info) return; await handleOpenInfo(info);
});

// Init
refreshRecents();
window.addEventListener('resize', updateImageTransform);

// Dark mode settings
function applyDark(d) { document.body.classList.toggle('dark', d); }
function getSettings() { try { return JSON.parse(localStorage.getItem('settings')||'{}'); } catch { return {} } }
function setSettings(obj) { localStorage.setItem('settings', JSON.stringify(obj)); }
const settings = getSettings();
const dark = !!settings.dark; applyDark(dark); if (darkChk) darkChk.checked = dark;
const savedFit = !!settings.fit; if (fitChk) fitChk.checked = savedFit;
const savedWrap = settings.wrap !== false; if (wrapChk) wrapChk.checked = savedWrap;
if (darkChk) darkChk.addEventListener('change', () => { const s=getSettings(); s.dark=darkChk.checked; setSettings(s); applyDark(darkChk.checked); });
if (fitChk) fitChk.addEventListener('change', () => { const s=getSettings(); s.fit=fitChk.checked; setSettings(s); });
if (wrapChk) wrapChk.addEventListener('change', () => { const s=getSettings(); s.wrap=wrapChk.checked; setSettings(s); applyWrap(wrapChk.checked); });

function applyWrap(on) { textEl.style.whiteSpace = on ? 'pre-wrap' : 'pre'; textEl.style.wordBreak = on ? 'break-word' : 'normal'; }
applyWrap(savedWrap);

function updateImageStatus() {
  if (imgEl.classList.contains(''hidden'')) return;
  const path = currentPath || '';
  const natW = imgEl.naturalWidth || 0;
  const natH = imgEl.naturalHeight || 0;
  let eff = imageZoom;
  if (fitChk.checked && natW && natH) {
    const cw = document.getElementById(''content'').clientWidth;
    const ch = document.getElementById(''content'').clientHeight;
    const sx = cw / natW; const sy = ch / natH; eff = Math.max(0.1, Math.min(6.0, Math.min(sx, sy)));
  }
  const estMB = ((natW * natH * 4) / (1024*1024)).toFixed(1);
  const fitNote = fitChk.checked ? ' Fit: on' : '';
  statusEl.textContent = `${path} — ${natW}x${natH} px — Zoom: ${(eff*100).toFixed(0)}% — Texture ~${estMB} MB${fitNote}`;
}

// Copy/Open
if (copyBtn) copyBtn.addEventListener('click', async () => { if (currentPath) { try { await window.api.copyText(currentPath); statusEl.textContent = 'Path copied to clipboard'; } catch {} } });
if (revealBtn) revealBtn.addEventListener('click', () => { if (currentPath) window.api.revealInFolder(currentPath); });
