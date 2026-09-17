<script>
  import { appState } from '../store.svelte.js';

  let activeTask = $derived(
    appState.activeTaskId ? appState.tasks.find(t => t.id === appState.activeTaskId) : null
  );

  let workspaceLabel = $derived.by(() => {
    if (appState.activeWorkspace === 'agent') return 'AI 助理對話';
    if (appState.activeWorkspace === 'sales') return '銷售與訂單';
    if (appState.activeWorkspace === 'settings') return '系統與市集管理';
    if (appState.activeWorkspace === 'finance') return '財務管理';
    if (appState.activeWorkspace === 'crm') return '客戶關係';
    const mod = appState.installedModules.find(m => m.id === appState.activeWorkspace);
    return mod ? mod.name : '業務模組';
  });

  let taskContextLabel = $derived(
    activeTask ? activeTask.title : null
  );
</script>

<div class="breadcrumb-container">
  <div class="breadcrumb-nav">
    <span class="breadcrumb-item module-name">{workspaceLabel}</span>
    {#if taskContextLabel}
      <span class="breadcrumb-separator">/</span>
      <span class="breadcrumb-item task-name font-bold">{taskContextLabel}</span>
    {/if}
  </div>

  <div class="breadcrumb-actions">
    <button class="icon-tool-btn" title="系統工具與偏好設定" aria-label="工具設定">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path>
      </svg>
    </button>
  </div>
</div>

<style>
  .breadcrumb-container {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 20px;
    height: 44px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-primary);
    flex-shrink: 0;
  }

  .breadcrumb-nav {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.88rem;
  }

  .module-name {
    color: var(--text-secondary);
  }

  .breadcrumb-separator {
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  .task-name {
    color: var(--text-primary);
    letter-spacing: 0.2px;
  }

  .breadcrumb-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .icon-tool-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--border-color);
    border-radius: 50%;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .icon-tool-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
    border-color: var(--border-active);
  }
</style>
