const openBtn = document.getElementById('open');
const filePick = document.getElementById('file-pick');
const recentSel = document.getElementById('recent');
const fitChk = document.getElementById('fit');
const wrapChk = document.getElementById('wrap');
const darkChk = document.getElementById('dark');
const zmMinus = document.getElementById('zm-');
const zmPlus = document.getElementById('zm+');
const z100 = document.getElementById('z100');
const clearBtn = document.getElementById('clear');
const textEl = document.getElementById('text');
const imgEl = document.getElementById('image');
const statusEl = document.getElementById('status');
const findInput = document.getElementById('find');
const matchesEl = document.getElementById('matches');
const copyBtn = document.getElementById('copy');
const revealBtn = document.getElementById('reveal');
const clearRecentsBtn = document.getElementById('clear-recents');
const prevBtn = document.getElementById('prev');
const nextBtn = document.getElementById('next');
const findContainer = document.getElementById('find-container');
let inBlockComment = false;
const hasApi = !!(window.api && typeof window.api.open === 'function');
const fsApiAvailable = typeof window.showOpenFilePicker === 'function' && !!window.indexedDB;

// IndexedDB helpers for persisting FileSystemFileHandle (browser mode)
async function idbOpen() {
  return await new Promise((resolve, reject) => {
    const req = indexedDB.open('viewer-db', 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains('handles')) db.createObjectStore('handles', { keyPath: 'id' });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error || new Error('indexedDB open failed'));
  });
}

async function idbPutHandle(id, handle, name) {
  const db = await idbOpen();
  await new Promise((resolve, reject) => {
    const tx = db.transaction('handles', 'readwrite');
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error || new Error('idb put failed'));
    tx.objectStore('handles').put({ id, handle, name });
  });
}

async function idbGetHandle(id) {
  const db = await idbOpen();
  return await new Promise((resolve, reject) => {
    const tx = db.transaction('handles', 'readonly');
    tx.onerror = () => reject(tx.error || new Error('idb get failed'));
    const req = tx.objectStore('handles').get(id);
    req.onsuccess = () => resolve(req.result || null);
    req.onerror = () => reject(req.error || new Error('idb req failed'));
  });
}

let currentPath = null;
let currentExt = '';
let imageZoom = 1.0;
let textZoom = 1.0;
const MAX_IMAGE_TEXTURE_BYTES = 128 * 1024 * 1024; // ~128MB

if (openBtn) openBtn.addEventListener('click', async () => {
  if (hasApi) {
    const info = await window.api.open();
    if (!info) return;
    handleOpenInfo(info);
    return;
  }
  if (fsApiAvailable) {
    try {
      const pickerOpts = {
        multiple: false,
        types: [
          { description: 'Images', accept: { 'image/*': ['.png','.jpg','.jpeg','.gif','.bmp','.webp'] } },
          { description: 'Text/Source', accept: { 'text/plain': ['.txt','.md','.json','.js','.ts','.py','.rs','.toml','.html','.css'] } }
        ]
      };
      const [handle] = await window.showOpenFilePicker(pickerOpts);
      if (!handle) return;
      const file = await handle.getFile();
      const name = file.name || 'untitled';
      const ext = (name.split('.').pop()||'').toLowerCase();
      const id = (window.crypto && crypto.randomUUID) ? crypto.randomUUID() : String(Date.now()) + Math.random();
      await idbPutHandle(id, handle, name);
      const IMG_EXTS = new Set(['png','jpg','jpeg','gif','bmp','webp']);
      if (IMG_EXTS.has(ext) || (file.type && file.type.startsWith('image/'))) {
        const url = URL.createObjectURL(file);
        await handleOpenInfo({ path: name, kind: 'image', url, handleId: id });
      } else {
        const text = await file.text();
        await handleOpenInfo({ path: name, kind: 'text', text, handleId: id });
      }
      return;
    } catch (e) {
      // User cancelled or API error; fall back
    }
  }
  if (filePick) filePick.click();
});

if (!hasApi) {
  // Hide Electron-only controls
  if (copyBtn) copyBtn.style.display = 'none';
  if (revealBtn) revealBtn.style.display = 'none';
}

async function handleOpenInfo(info) {
  if (info.kind === 'error') { alert(info.error); return; }
  currentPath = info.path;
  currentExt = (currentPath.split('.').pop()||'').toLowerCase();
  if (info.kind === 'text') {
    imgEl.classList.add('hidden');
    textEl.classList.remove('hidden');
    findContainer.classList.remove('hidden');
    statusEl.textContent = `${info.path} - ${info.text.split(/\n/).length} lines`;
    if (info.handleId) addRecentHandle(info.path, info.handleId); else addRecent(info.path);
    renderText(info.text, currentExt, findInput.value);
    // re-apply wrap and zoom on new content
    const st = getSettings();
    applyWrap(st.wrap !== false);
    applyTextZoom(st.textZoom || 1.0);
  } else if (info.kind === 'image') {
    textEl.classList.add('hidden');
    imgEl.classList.remove('hidden');
    findContainer.classList.add('hidden');
    imageZoom = 1.0;
    fitChk.checked = false;
    imgEl.style.transform = `scale(${imageZoom})`;
    imgEl.src = info.url || info.path;
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
      // Auto-fit if the image is larger than the viewport
      const content = document.getElementById('content');
      const cw = content.clientWidth;
      const ch = content.clientHeight;
      const sx = cw / (imgEl.naturalWidth || 1);
      const sy = ch / (imgEl.naturalHeight || 1);
      const s = Math.max(0.1, Math.min(6.0, Math.min(sx, sy)));
      fitChk.checked = s < 1.0;
      updateImageStatus();
      const _ = content.offsetHeight;
      updateImageTransform();
    };
    if (info.handleId) addRecentHandle(info.path, info.handleId); else addRecent(info.path);
  }
  updateActionsEnabled();
}

function clearView() {
    currentPath = null;
    textEl.classList.add('hidden');
    textEl.innerHTML = '';
    imgEl.classList.add('hidden');
    imgEl.src = '';
    statusEl.textContent = '';
    findContainer.classList.add('hidden');
    updateActionsEnabled();
}

function updateActionsEnabled() {
    const hasPath = currentPath !== null;
    if (copyBtn) copyBtn.disabled = !hasApi || !hasPath;
    if (revealBtn) revealBtn.disabled = !hasApi || !hasPath;
}

if (clearBtn) clearBtn.addEventListener('click', clearView);

function updateImageTransform() {
  if (fitChk.checked && imgEl.naturalWidth && imgEl.naturalHeight) {
    const cw = document.getElementById('content').clientWidth;
    const ch = document.getElementById('content').clientHeight;
    const sx = cw / imgEl.naturalWidth;
    const sy = ch / imgEl.naturalHeight;
    const s = Math.max(0.1, Math.min(6.0, Math.min(sx, sy)));
    imgEl.style.transform = `scale(${s})`;
    imgEl.style.transformOrigin = 'top left';
  } else {
    imgEl.style.transform = `scale(${imageZoom})`;
    imgEl.style.transformOrigin = 'top left';
  }
  updateImageStatus();
  try { const s = getSettings(); s.imageZoom = imageZoom; setSettings(s); } catch {}
}

if (fitChk) fitChk.addEventListener('change', () => updateImageTransform());
if (zmMinus) zmMinus.addEventListener('click', () => { if (fitChk) fitChk.checked = false; imageZoom = Math.max(0.1, imageZoom/1.1); updateImageTransform(); });
if (zmPlus) zmPlus.addEventListener('click', () => { if (fitChk) fitChk.checked = false; imageZoom = Math.min(6.0, imageZoom*1.1); updateImageTransform(); });
if (z100) z100.addEventListener('click', () => { if (fitChk) fitChk.checked = false; imageZoom = 1.0; updateImageTransform(); });

const contentEl = document.getElementById('content');
if (contentEl) contentEl.addEventListener('wheel', (e) => {
  if (e.ctrlKey) {
      if (textEl.classList.contains('hidden')) return;
      e.preventDefault();
      const newZoom = Math.max(0.6, Math.min(3.0, textZoom * (e.deltaY < 0 ? 1.1 : 1/1.1)));
      applyTextZoom(newZoom);
      return;
  }
  if (imgEl.classList.contains('hidden')) return;
  if (!imgEl.matches(':hover')) return;
  e.preventDefault();
  fitChk.checked = false;
  imageZoom = Math.max(0.1, Math.min(6.0, imageZoom * (e.deltaY < 0 ? 1.1 : 1/1.1)));
  updateImageTransform();
}, { passive: false });

if (findInput) findInput.addEventListener('input', () => {
  if (!textEl.classList.contains('hidden')) {
    renderText(textEl.textContent, currentExt, findInput.value);
  }
});

// Find navigation
let findIndex = 0;
if (nextBtn) nextBtn.addEventListener('click', () => stepFind(1));
if (prevBtn) prevBtn.addEventListener('click', () => stepFind(-1));
if (findInput) findInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') stepFind(1); });

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
    let pos = 0;
    count = 0;
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

  let out = '';
  let i = 0;
  while (i < line.length) {
    const ch = line[i];

    if (ext === 'js' && ch === '/' && line[i+1] === '*') {
        const end = line.indexOf('*/', i + 2);
        if (end !== -1) {
            out += com(line.substring(i, end + 2));
            i = end + 2;
        } else {
            out += com(line.substring(i));
            i = line.length;
        }
        continue;
    }

    if ((ext === 'js' || ext === 'rs') && ch === '/' && line[i+1] === '/') {
        out += com(line.substring(i));
        i = line.length;
        continue;
    }

    if ((ext === 'py' || ext === 'toml') && ch === '#') {
        out += com(line.substring(i));
        i = line.length;
        continue;
    }

    if (ch === '"') {
        let j = i + 1;
        while (j < line.length && line[j] !== '"') j++;
        out += str(line.substring(i, j + 1));
        i = j + 1;
        continue;
    }

    if (/W/.test(ch)) {
        let buf = '';
        let j = i;
        while (j < line.length && /W/.test(line[j])) {
            buf += line[j];
            j++;
        }
        const lc = buf.toLowerCase();
        if (ext === 'py' && PY_KW.has(buf)) out += kw(buf);
        else if (ext === 'rs' && RS_KW.has(buf)) out += kw(buf);
        else if (ext === 'js' && JS_KW.has(buf)) out += kw(buf);
        else if (lc === 'true' || lc === 'false' || lc === 'null' || lc === 'none') out += boolc(buf);
        else if (/^\d+$/.test(buf)) out += num(buf);
        else out += base(buf);
        i = j;
        continue;
    }

    if (ch === '(' || ch === '[' || ch === '}') {
        out += bracket(ch, true);
        i++;
        continue;
    }

    if (ch === ')' || ch === ']' || ch === '{') {
        out += bracket(ch, false);
        i++;
        continue;
    }

    out += base(ch);
    i++;
  }
  return out;
}

const RS_KW = new Set(['as','async','await','break','const','continue','crate','dyn','else','enum','extern','false','fn','for','if','impl','in','let','loop','match','mod','move','mut','pub','ref','return','self','Self','static','struct','super','trait','true','type','unsafe','use','where','while','union','box','try','yield','macro','macro_rules']);
const PY_KW = new Set(['False','None','True','and','as','assert','async','await','break','class','continue','def','del','elif','else','except','finally','for','from','global','if','import','in','is','lambda','nonlocal','not','or','pass','raise','return','try','while','with','yield','match','case']);
const JS_KW = new Set(['function','const','let','var','class','extends','super','import','export','default','return','if','else','for','while','do','switch','case','break','continue','try','catch','finally','throw','new','this','in','of','await','async','yield','typeof','instanceof','void','delete','with']);

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
    const sep = document.createElement('option');
    sep.disabled = true;
    sep.textContent = '──────────';
    recentSel.appendChild(sep);
    const clearOpt = document.createElement('option');
    clearOpt.value = '--clear--';
    clearOpt.textContent = 'Clear Recents';
    recentSel.appendChild(clearOpt);
  }
}
// Replaced below with a smarter handler that supports browser file handles
recentSel.addEventListener('change', async () => {});

// Init
// Override recents helpers to support browser file handles and Electron paths
function getRecents() {
  try {
    const raw = JSON.parse(localStorage.getItem('recents')||'[]');
    return raw.map((it) => typeof it === 'string' ? { type: hasApi ? 'path' : 'name', value: it, label: it } : it);
  } catch { return [] }
}
function setRecents(list) { localStorage.setItem('recents', JSON.stringify(list.slice(-10))); refreshRecents(); }
function addRecentPath(p) {
  const rec = getRecents().filter(x => !(x.type === 'path' && x.value === p));
  rec.push({ type: 'path', value: p, label: p });
  setRecents(rec);
}
function addRecentHandle(name, id) {
  const rec = getRecents().filter(x => !(x.type === 'handle' && x.value === id));
  rec.push({ type: 'handle', value: id, label: name });
  setRecents(rec);
}
function addRecentNameOnly(name) {
  const rec = getRecents().filter(x => !(x.type === 'name' && x.value === name));
  rec.push({ type: 'name', value: name, label: name });
  setRecents(rec);
}
function addRecent(p) { if (hasApi) addRecentPath(p); else addRecentNameOnly(p); }
function refreshRecents() {
  const rec = getRecents();
  recentSel.innerHTML = '';
  if (!rec.length) { const opt = document.createElement('option'); opt.value=''; opt.textContent='(empty)'; recentSel.appendChild(opt); }
  else {
    const opt0 = document.createElement('option'); opt0.value=''; opt0.textContent='Recents'; recentSel.appendChild(opt0);
    for (let i=rec.length-1;i>=0;i--) {
      const item = rec[i];
      const opt = document.createElement('option');
      opt.value = JSON.stringify({ t: item.type, v: item.value });
      opt.textContent = item.label || item.value;
      recentSel.appendChild(opt);
    }
    const sep = document.createElement('option'); sep.disabled = true; sep.textContent = '----------------'; recentSel.appendChild(sep);
    const clearOpt = document.createElement('option'); clearOpt.value='--clear--'; clearOpt.textContent='Clear Recents'; recentSel.appendChild(clearOpt);
  }
}

recentSel.onchange = async () => {
  const val = recentSel.value;
  if (!val) return;
  if (val === '--clear--') { setRecents([]); return; }
  try {
    // Values are JSON objects with {t:'path', v:'...'} when built; also support plain path strings
    let pathVal = val;
    if (val.startsWith('{')) { const parsed = JSON.parse(val); pathVal = parsed.v || ''; }
    if (!pathVal) { recentSel.value=''; return; }
    const info = await window.api.openPath(pathVal);
    if (!info) return; await handleOpenInfo(info);
  } catch {
    recentSel.value='';
  }
};

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

function applyTextZoom(level) {
    textZoom = level;
    textEl.style.fontSize = `${textZoom * 100}%`;
    const s = getSettings();
    s.textZoom = textZoom;
    setSettings(s);
}

function updateImageStatus() {
  if (imgEl.classList.contains('hidden')) return;
  const path = currentPath || '';
  const natW = imgEl.naturalWidth || 0;
  const natH = imgEl.naturalHeight || 0;
  let eff = imageZoom;
  if (fitChk.checked && natW && natH) {
    const cw = document.getElementById('content').clientWidth;
    const ch = document.getElementById('content').clientHeight;
    const sx = cw / natW; const sy = ch / natH; eff = Math.max(0.1, Math.min(6.0, Math.min(sx, sy)));
  }
  const estMB = ((natW * natH * 4) / (1024*1024)).toFixed(1);
  const fitNote = fitChk.checked ? ' Fit: on' : '';
  statusEl.textContent = `${path} — ${natW}x${natH} px — Zoom: ${(eff*100).toFixed(0)}% — Texture ~${estMB} MB${fitNote}`;
}

// Copy/Open
if (copyBtn) copyBtn.addEventListener('click', async () => { if (currentPath) { try { await window.api.copyText(currentPath); statusEl.textContent = 'Path copied to clipboard'; } catch {} } });
if (revealBtn) revealBtn.addEventListener('click', () => { if (currentPath) window.api.revealInFolder(currentPath); });

// Browser fallback removed in Electron-only build

// Line numbers
const lnChk = document.getElementById('linenumbers');
function applyLN(on) { textEl.classList.toggle('ln', on); textEl.classList.toggle('ln-off', !on); }
const savedLN = settings.ln !== false;
applyLN(savedLN);
if (lnChk) {
  lnChk.checked = savedLN;
  lnChk.addEventListener('change', () => {
    const s = getSettings(); s.ln = lnChk.checked; setSettings(s);
    applyLN(lnChk.checked);
    // re-render to show/hide numbers
    if (!textEl.classList.contains('hidden')) {
      renderText(textEl.textContent, currentExt, findInput.value);
    }
  });
}

updateActionsEnabled();

// Clear Recents button
if (clearRecentsBtn) clearRecentsBtn.addEventListener('click', async () => {
  setRecents([]);
});

// Hotkeys
document.addEventListener('keydown', async (e) => {
  const ctrlOrCmd = e.ctrlKey || e.metaKey;
  // Focus recents: Ctrl/Cmd + Shift + O
  if (ctrlOrCmd && e.shiftKey && (e.key === 'O' || e.key === 'o')) {
    e.preventDefault();
    if (recentSel) {
      recentSel.focus();
      // best-effort visual hint
      recentSel.style.outline = '2px solid #e06c75';
      setTimeout(() => { recentSel.style.outline = ''; }, 600);
    }
    return;
  }
  // Open recent by index: Ctrl/Cmd + Alt + [1..9]
  if (ctrlOrCmd && e.altKey && /^Digit[1-9]$/.test(e.code)) {
    e.preventDefault();
    const idx = parseInt(e.code.replace('Digit',''), 10); // 1..9
    const rec = getRecents();
    if (!rec.length) return;
    if (idx > rec.length) return; // nothing for that slot
    const item = rec[rec.length - idx]; // latest is highest index
    try {
      if (item.type === 'path' && hasApi) {
        const info = await window.api.openPath(item.value);
        if (info) await handleOpenInfo(info);
      } else if (item.type === 'handle' && !hasApi && fsApiAvailable) {
        const saved = await idbGetHandle(item.value);
        if (!saved || !saved.handle) return;
        const perm = await saved.handle.queryPermission({ mode: 'read' });
        if (perm !== 'granted') {
          const p2 = await saved.handle.requestPermission({ mode: 'read' });
          if (p2 !== 'granted') return;
        }
        const file = await saved.handle.getFile();
        const name = saved.name || file.name;
        const ext = (name.split('.').pop()||'').toLowerCase();
        if (file.type.startsWith('image/') || ['png','jpg','jpeg','gif','bmp','webp'].includes(ext)) {
          const url = URL.createObjectURL(file);
          await handleOpenInfo({ path: name, kind: 'image', url, handleId: item.value });
        } else {
          const text = await file.text();
          await handleOpenInfo({ path: name, kind: 'text', text, handleId: item.value });
        }
      }
    } catch {}
  }
});
