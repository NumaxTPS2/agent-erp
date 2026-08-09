import { invoke } from './tauri.js';
import { appState } from './store.svelte.js';

export async function loadModule(moduleId) {
  // CRM is loaded as a standalone HTML page in an iframe, so it does not need JS mounting
  if (moduleId === 'crm') return;

  console.log(`[Registry] Attempting to read source for module: ${moduleId}`);
  try {
    // 1. Retrieve raw source string via Tauri IPC (bypasses CORS & mixed-protocol blocks)
    const source = await invoke('get_module_source', { moduleId });
    
    // 2. Create Blob URL under same host origin (e.g. blob:http://localhost:5173/...)
    const blob = new Blob([source], { type: 'application/javascript' });
    const blobUrl = URL.createObjectURL(blob);
    
    console.log(`[Registry] Dynamic importing Blob URL for: ${moduleId}`);
    const module = await import(/* @vite-ignore */ blobUrl);
    appState.loadedComponents[moduleId] = module.default;
    
    // Revoke Blob URL to free up memory
    URL.revokeObjectURL(blobUrl);
    
    // Explicitly re-assign to trigger Svelte 5 reactivity updates
    appState.loadedComponents = { ...appState.loadedComponents };
    
    console.log(`[Registry] Successfully loaded dynamic module component: ${moduleId}`);
  } catch (err) {
    console.error(`[Registry] Failed to load module component: ${moduleId}`, err);
  }
}
