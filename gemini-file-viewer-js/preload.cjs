const { contextBridge, ipcRenderer, clipboard, shell } = require('electron');

contextBridge.exposeInMainWorld('api', {
  open: () => ipcRenderer.invoke('dialog:open'),
  openPath: (p) => ipcRenderer.invoke('open:path', p),
  copyText: (t) => clipboard.writeText(t || ''),
  revealInFolder: (p) => shell.showItemInFolder(p)
});
