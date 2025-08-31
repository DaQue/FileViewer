import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('api', {
  open: () => ipcRenderer.invoke('dialog:open'),
  openPath: (p) => ipcRenderer.invoke('open:path', p)
});
