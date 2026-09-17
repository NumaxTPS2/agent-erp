<script>
  import { appState, setTaskPanelCollapsed } from '../store.svelte.js';

  let pendingTasksCount = $derived(
    appState.tasks.filter(t => t.status === 'pending' || t.status === 'in_progress').length
  );

  let unreadNotificationsCount = $derived(
    appState.notifications.length
  );

  // Sample or live notifications matching system design
  let notificationsList = $derived.by(() => {
    if (appState.notifications.length > 0) {
      return appState.notifications.map(n => ({
        id: n.id,
        level: n.level || 'alert',
        dot: n.level === 'warning' ? '🟡' : '🔴',
        title: n.title,
        message: n.message,
        time: n.time || '剛剛'
      }));
    }
    return [];
  });

  let isInstitutionsCollapsed = $state(true);

  function handleTaskStatClick() {
    if (appState.taskPanelCollapsed) {
      setTaskPanelCollapsed(false);
    }
  }
</script>

<aside class="overview-sidebar-container">
  <div class="overview-header">
    <span class="overview-title">現況總覽</span>
  </div>

  <div class="overview-scroll">
    <!-- Top Stat Tiles -->
    <div class="stat-tiles-row">
      <button 
        type="button" 
        class="stat-tile clickable-tile" 
        onclick={handleTaskStatClick}
        title="點擊展開左側任務面板"
        aria-label="檢視待處理任務"
      >
        <span class="stat-number amber">{pendingTasksCount}</span>
        <span class="stat-label">待處理任務</span>
      </button>
      <div class="stat-tile">
        <span class="stat-number cyan">{unreadNotificationsCount}</span>
        <span class="stat-label">未讀通知</span>
      </div>
    </div>

    <!-- Notifications Section -->
    <div class="section-group">
      <div class="section-title">通知</div>
      {#if notificationsList.length === 0}
        <div class="empty-notif-box">
          <span class="empty-notif-text">目前無未讀通知或警報</span>
        </div>
      {:else}
        <div class="notif-cards-list">
          {#each notificationsList as notif (notif.id)}
            <div class="notif-card">
              <div class="notif-card-header">
                <span class="notif-dot">{notif.dot}</span>
                <span class="notif-card-title">{notif.title}</span>
              </div>
              <p class="notif-card-desc">
                {notif.message}{notif.time ? ` · ${notif.time}` : ''}
              </p>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Other Public Institutions Section (Collapsible) -->
    <div class="section-group">
      <div class="section-header-row">
        <button 
          type="button" 
          class="collapse-toggle-btn" 
          onclick={() => isInstitutionsCollapsed = !isInstitutionsCollapsed}
          title={isInstitutionsCollapsed ? '展開情境示意' : '收合情境示意'}
        >
          <span class="section-title">其他往來公家單位</span>
          <span class="badge-mock">情境示意</span>
          <svg class="chevron-icon {isInstitutionsCollapsed ? '' : 'open'}" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9"></polyline>
          </svg>
        </button>
      </div>

      {#if !isInstitutionsCollapsed}
        <p class="section-caption">
          下列客戶僅示意「不只一家往來單位」，未做滿三段角色流程（中山國小已有獨立的新任務示範）。
        </p>

        <div class="client-card">
          <div class="client-icon">🏛️</div>
          <div class="client-info">
            <span class="client-name">台北市政府</span>
            <span class="client-type">共同供應契約客戶</span>
          </div>
        </div>
      {/if}
    </div>
  </div>
</aside>

<style>
  .overview-sidebar-container {
    width: 290px;
    height: 100vh;
    background: #15161E;
    border-left: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .overview-header {
    padding: 12px 18px;
    border-bottom: 1px solid var(--border-color);
    height: 44px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .overview-title {
    font-size: 0.92rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .overview-scroll {
    flex-grow: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* Stat Tiles */
  .stat-tiles-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .stat-tile {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 16px 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    transition: all var(--transition-fast);
  }

  .stat-tile:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(255, 255, 255, 0.15);
  }

  .stat-number {
    font-size: 1.75rem;
    font-weight: 700;
    line-height: 1.1;
  }

  .stat-number.amber {
    color: rgb(var(--accent-amber));
  }

  .stat-number.cyan {
    color: rgb(var(--accent-cyan));
  }

  .stat-label {
    font-size: 0.78rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  /* Sections */
  .section-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .section-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .section-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .stat-tile.clickable-tile {
    cursor: pointer;
    border: 1px solid var(--border-color);
    outline: none;
    font-family: inherit;
    text-align: center;
  }

  .stat-tile.clickable-tile:hover {
    background: rgba(245, 158, 11, 0.08);
    border-color: rgba(245, 158, 11, 0.4);
    transform: translateY(-1px);
  }

  .collapse-toggle-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    color: inherit;
    gap: 8px;
    font-family: inherit;
  }

  .chevron-icon {
    color: var(--text-muted);
    transition: transform var(--transition-fast);
  }

  .chevron-icon.open {
    transform: rotate(180deg);
  }

  .badge-mock {
    font-size: 0.68rem;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-muted);
    border: 1px solid var(--border-color);
    font-weight: 500;
    margin-left: auto;
  }

  .section-caption {
    font-size: 0.75rem;
    color: var(--text-muted);
    line-height: 1.45;
  }

  /* Empty Notification Box */
  .empty-notif-box {
    background: rgba(255, 255, 255, 0.02);
    border: 1px dashed var(--border-color);
    border-radius: var(--radius-sm);
    padding: 16px 12px;
    text-align: center;
  }

  .empty-notif-text {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  /* Notifications */
  .notif-cards-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .notif-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    transition: all var(--transition-fast);
  }

  .notif-card:hover {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(255, 255, 255, 0.15);
  }

  .notif-card-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .notif-dot {
    font-size: 0.75rem;
    line-height: 1;
  }

  .notif-card-title {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .notif-card-desc {
    font-size: 0.75rem;
    color: var(--text-secondary);
    line-height: 1.35;
    padding-left: 18px;
  }

  /* Client Card */
  .client-card {
    display: flex;
    align-items: center;
    gap: 12px;
    background: rgba(255, 255, 255, 0.025);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
  }

  .client-icon {
    font-size: 1.2rem;
  }

  .client-info {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-grow: 1;
  }

  .client-name {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .client-type {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
</style>
