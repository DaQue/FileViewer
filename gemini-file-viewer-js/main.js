import { app, BrowserWindow, dialog, ipcMain } from 'electron';
import path from 'node:path';
import fs from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

const IMG_EXTS = new Set(['png','jpg','jpeg','gif','bmp','webp']);

async function createWindow() {
  const win = new BrowserWindow({
    width: 1000,
    height: 700,
    webPreferences: {
      preload: path.join(process.cwd(), 'preload.cjs')
    },
    title: 'Gemini File Viewer (JS)'
  });

  await win.loadFile('renderer/index.html');
}

ipcMain.handle('dialog:open', async () => {
  const win = BrowserWindow.getFocusedWindow();
  const { canceled, filePaths } = await dialog.showOpenDialog(win, {
    properties: ['openFile'],
    filters: [
      { name: 'All Supported', extensions: ['txt','rs','py','toml','md','json','js','html','css','png','jpg','jpeg','gif','bmp','webp'] },
      { name: 'Images', extensions: ['png','jpg','jpeg','gif','bmp','webp'] },
      { name: 'Text/Source', extensions: ['txt','rs','py','toml','md','json','js','html','css'] }
    ]
  });
  if (canceled || !filePaths[0]) return null;
  const filePath = filePaths[0];
  const ext = path.extname(filePath).slice(1).toLowerCase();
  if (IMG_EXTS.has(ext)) {
    return { path: filePath, url: String(pathToFileURL(filePath)), kind: 'image' };
  } else {
    try {
      const data = await fs.readFile(filePath);
      const text = new TextDecoder('utf-8', { fatal: false }).decode(data);
      return { path: filePath, kind: 'text', text };
    } catch (e) {
      return { path: filePath, kind: 'error', error: String(e) };
    }
  }
});

ipcMain.handle('open:path', async (_e, filePath) => {
  try {
    const ext = path.extname(filePath).slice(1).toLowerCase();
    if (IMG_EXTS.has(ext)) {
      return { path: filePath, url: String(pathToFileURL(filePath)), kind: 'image' };
    } else {
      const data = await fs.readFile(filePath);
      const text = new TextDecoder('utf-8', { fatal: false }).decode(data);
      return { path: filePath, kind: 'text', text };
    }
  } catch (e) {
    return { path: filePath, kind: 'error', error: String(e) };
  }
});

app.whenReady().then(() => {
  createWindow();
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
