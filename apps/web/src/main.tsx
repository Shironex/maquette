import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { IN_TAURI } from './ipc/runtime';
import './index.css';

async function start() {
  // Mock mode: outside Tauri during development, answer IPC from memory.
  if (!IN_TAURI && import.meta.env.DEV) await import('./ipc/mock/install');
  const { App } = await import('./App');
  const root = document.getElementById('root');
  if (!root) throw new Error('The #root element is missing from index.html.');
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>
  );
}

void start();
