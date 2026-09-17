<script>
  import { appState, switchActiveTask, setTaskPanelCollapsed } from '../store.svelte.js';

  let isCollapsed = $derived(appState.taskPanelCollapsed);

  let pendingCount = $derived(
    appState.tasks.filter(t => t.status === 'pending' || t.status === 'in_progress').length
  );

  // Group tasks into hierarchy: top-level parents and their children
  let parentTasks = $derived(
    appState.tasks.filter(t => !t.parent_task_id)
  );

  let getChildrenOf = $derived((parentId) => {
    return appState.tasks.filter(t => t.parent_task_id === parentId);
  });

  // Any orphaned child tasks whose parent is not in parentTasks
  let orphanTasks = $derived(
    appState.tasks.filter(t => t.parent_task_id && !appState.tasks.some(p => p.id === t.parent_task_id))
  );

  function handleSelectTask(taskId) {
    if (appState.activeTaskId === taskId) {
      // Toggle back to main if clicking same active task
      switchActiveTask(null);
    } else {
      switchActiveTask(taskId);
    }
  }

  function formatCompletedTime(timestamp) {
    if (!timestamp) return '';
    const date = new Date(timestamp * 1000);
    const hours = String(date.getHours()).padStart(2, '0');
    const mins = String(date.getMinutes()).padStart(2, '0');
    return `${hours}:${mins}`;
  }

  function getStatusBadge(status) {
    switch (status) {
      case 'in_progress':
        return { label: 'IN_PROGRESS', class: 'badge-in-progress' };
      case 'pending':
        return { label: 'PENDING', class: 'badge-pending' };
      case 'done':
        return { label: 'DONE', class: 'badge-done' };
      case 'cancelled':
        return { label: 'CANCELLED', class: 'badge-cancelled' };
      default:
        return { label: status.toUpperCase(), class: 'badge-pending' };
    }
  }
</script>

<aside class="task-panel-container {isCollapsed ? 'collapsed' : ''}">
  {#if !isCollapsed}
    <div class="task-panel-header">
      <div class="header-left">
        <svg class="checklist-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 11l3 3L22 4"></path>
          <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"></path>
        </svg>
        <span class="panel-title">我的任務</span>
        {#if pendingCount > 0}
          <span class="badge-count">{pendingCount}</span>
        {/if}
      </div>

      <button 
        class="collapse-btn" 
        onclick={() => setTaskPanelCollapsed(true)}
        title="收合任務面板" 
        aria-label="收合任務面板"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="11 17 6 12 11 7"></polyline>
          <polyline points="18 17 13 12 18 7"></polyline>
        </svg>
      </button>
    </div>

    <div class="task-list-scroll">
      {#if appState.tasks.length === 0}
        <div class="empty-tasks-box">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
          <p>目前尚無任務</p>
        </div>
      {:else}
        {#each parentTasks as parent (parent.id)}
          {@const parentBadge = getStatusBadge(parent.status)}
          {@const children = getChildrenOf(parent.id)}
          <div class="task-group">
            <!-- Parent Task Card -->
            <button 
              type="button" 
              class="task-card parent-card {appState.activeTaskId === parent.id ? 'active' : ''} {parent.status === 'done' ? 'card-done' : ''}"
              onclick={() => handleSelectTask(parent.id)}
            >
              <div class="card-top-row">
                <span class="task-card-title">{parent.title}</span>
                <span class="status-badge {parentBadge.class}">{parentBadge.label}</span>
              </div>
              <div class="card-meta-row">
                <span>父任務 · {parent.assignee || '主管'}</span>
                {#if parent.status === 'done' && parent.completed_at}
                  <span> · {formatCompletedTime(parent.completed_at)} 完成</span>
                {/if}
              </div>
            </button>

            <!-- Subtask Cards -->
            {#if children.length > 0}
              <div class="subtasks-container">
                {#each children as child (child.id)}
                  {@const childBadge = getStatusBadge(child.status)}
                  <button 
                    type="button" 
                    class="task-card child-card {appState.activeTaskId === child.id ? 'active' : ''} {child.status === 'done' ? 'card-done' : ''}"
                    onclick={() => handleSelectTask(child.id)}
                  >
                    <div class="card-top-row">
                      <span class="task-card-title">{child.title}</span>
                      <span class="status-badge {childBadge.class}">{childBadge.label}</span>
                    </div>
                    <div class="card-meta-row">
                      <span>指派給：{child.assignee || '團隊成員'}</span>
                      {#if child.status === 'done' && child.completed_at}
                        <span> · {formatCompletedTime(child.completed_at)} 完成</span>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/each}

        <!-- Orphan Tasks if any -->
        {#each orphanTasks as orphan (orphan.id)}
          {@const orphanBadge = getStatusBadge(orphan.status)}
          <div class="task-group">
            <button 
              type="button" 
              class="task-card child-card {appState.activeTaskId === orphan.id ? 'active' : ''} {orphan.status === 'done' ? 'card-done' : ''}"
              onclick={() => handleSelectTask(orphan.id)}
            >
              <div class="card-top-row">
                <span class="task-card-title">{orphan.title}</span>
                <span class="status-badge {orphanBadge.class}">{orphanBadge.label}</span>
              </div>
              <div class="card-meta-row">
                <span>指派給：{orphan.assignee}</span>
                {#if orphan.status === 'done' && orphan.completed_at}
                  <span> · {formatCompletedTime(orphan.completed_at)} 完成</span>
                {/if}
              </div>
            </button>
          </div>
        {/each}
      {/if}
    </div>
  {:else}
    <div class="collapsed-strip">
      <button 
        class="expand-btn" 
        onclick={() => setTaskPanelCollapsed(false)}
        title="展開任務面板" 
        aria-label="展開任務面板"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="13 17 18 12 13 7"></polyline>
          <polyline points="6 17 11 12 6 7"></polyline>
        </svg>
      </button>

      <div class="collapsed-icon-wrap" onclick={() => setTaskPanelCollapsed(false)} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && setTaskPanelCollapsed(false)}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M9 11l3 3L22 4"></path>
          <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"></path>
        </svg>
        {#if pendingCount > 0}
          <span class="badge-count small">{pendingCount}</span>
        {/if}
      </div>

      <span class="collapsed-vertical-text">任務面板</span>
    </div>
  {/if}
</aside>

<style>
  .task-panel-container {
    width: 250px;
    height: 100vh;
    background: #15161E;
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
  }

  .task-panel-container.collapsed {
    width: 44px;
  }

  .task-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-color);
    height: 44px;
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .checklist-icon {
    color: var(--text-secondary);
  }

  .panel-title {
    font-size: 0.92rem;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.2px;
  }

  .badge-count {
    background: rgb(var(--accent-amber));
    color: #0F0F12;
    font-size: 0.72rem;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 9999px;
    line-height: 1.3;
  }

  .badge-count.small {
    position: absolute;
    top: -4px;
    right: -4px;
    padding: 0 4px;
    font-size: 0.65rem;
  }

  .collapse-btn, .expand-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: all var(--transition-fast);
  }

  .collapse-btn:hover, .expand-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  .task-list-scroll {
    flex-grow: 1;
    overflow-y: auto;
    padding: 12px 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .task-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .subtasks-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-left: 10px;
    border-left: 2px solid rgba(255, 255, 255, 0.06);
    margin-left: 6px;
    margin-top: 2px;
  }

  .task-card {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    text-align: left;
    cursor: pointer;
    transition: all var(--transition-fast);
    width: 100%;
  }

  .task-card:hover {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(var(--accent-rgb), 0.3);
    transform: translateY(-1px);
  }

  .task-card.active {
    background: rgba(var(--accent-rgb), 0.08);
    border-color: var(--accent);
    box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.15);
  }

  .task-card.card-done {
    opacity: 0.65;
  }

  .card-top-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .task-card-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-meta-row {
    font-size: 0.73rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-badge {
    font-size: 0.68rem;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    flex-shrink: 0;
  }

  .badge-in-progress {
    background: rgba(34, 211, 238, 0.15);
    color: rgb(34, 211, 238);
    border: 1px solid rgba(34, 211, 238, 0.3);
  }

  .badge-pending {
    background: rgba(245, 158, 11, 0.15);
    color: rgb(245, 158, 11);
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .badge-done {
    background: rgba(16, 185, 129, 0.15);
    color: rgb(16, 185, 129);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .badge-cancelled {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-muted);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .empty-tasks-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 30px 10px;
    color: var(--text-muted);
    gap: 8px;
    font-size: 0.85rem;
  }

  /* Collapsed minimal strip */
  .collapsed-strip {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 10px 4px;
    gap: 16px;
    height: 100%;
  }

  .collapsed-icon-wrap {
    position: relative;
    cursor: pointer;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    transition: all var(--transition-fast);
  }

  .collapsed-icon-wrap:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }

  .collapsed-vertical-text {
    writing-mode: vertical-lr;
    text-orientation: mixed;
    font-size: 0.78rem;
    color: var(--text-muted);
    letter-spacing: 2px;
    margin-top: 10px;
  }
</style>
