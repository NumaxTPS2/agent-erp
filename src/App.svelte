<script>
  import { onMount } from 'svelte';
  import { listen, isTauri } from './lib/tauri.js';
  import { 
    appState, 
    fetchOrders, 
    fetchAuditLogs, 
    fetchInstalledModules, 
    triggerWebhookSimulation,
    checkForUpdates,
    installUpdate,
    fetchModulesGallery,
    installModuleAction,
    uninstallModuleAction,
    navigate
  } from './lib/store.svelte.js';
  import ChatBox from './lib/components/ChatBox.svelte';
  import MutationDialog from './lib/components/MutationDialog.svelte';

  let activeTab = $derived(appState.activeWorkspace);
  let activeOrderFilter = $state('all');
  let selectedOrderId = $state(null);
  let isNotificationOpen = $state(false);

  // Initialize data on mount
  onMount(async () => {
    if (isTauri) {
      try {
        const { getVersion } = await import('@tauri-apps/api/app');
        appState.version = await getVersion();
      } catch (err) {
        console.warn("Failed to fetch version from Tauri:", err);
      }
    } else {
      appState.version = '0.1.0';
    }

    await fetchOrders();
    await fetchAuditLogs();
    await fetchInstalledModules();
    await fetchModulesGallery();

    // Hash router synchronization
    const handleHashChange = () => {
      const hash = location.hash.slice(1);
      if (!hash) {
        navigate('/app/sales');
      } else {
        appState.route = hash;
      }
    };
    window.addEventListener('hashchange', handleHashChange);
    // Initial sync
    handleHashChange();

    // Automatically listen to the background notification event from Rust
    const unlisten = await listen('notification-hub', (event) => {
      // Append to local notifications list
      appState.notifications.unshift({
        id: event.payload.id,
        title: event.payload.title,
        message: event.payload.message,
        time: 'Just now'
      });

      // Fire browser-standard desktop notification which Tauri webview routes to OS natively
      if (Notification.permission === 'granted') {
        new Notification(event.payload.title, { body: event.payload.message });
      } else if (Notification.permission !== 'denied') {
        Notification.requestPermission().then(permission => {
          if (permission === 'granted') {
            new Notification(event.payload.title, { body: event.payload.message });
          }
        });
      }

      // Refresh orders in real-time
      fetchOrders();
    });

    // Request notification permission early
    if (Notification.permission !== 'granted' && Notification.permission !== 'denied') {
      Notification.requestPermission();
    }

    return () => {
      if (typeof unlisten === 'function') unlisten();
      window.removeEventListener('hashchange', handleHashChange);
    };
  });

  function selectWorkspace(ws) {
    if (ws === 'sales') navigate('/app/sales');
    else if (ws === 'finance') navigate('/app/finance');
    else if (ws === 'crm') navigate('/app/crm');
    else if (ws === 'settings') navigate('/app/settings');
    else navigate(`/app/${ws}`);
  }

  // Svelte Action to safely mount dynamic vanilla components to a physical DIV node
  function mountModule(node, moduleId) {
    const ComponentConstructor = appState.loadedComponents[moduleId];
    if (!ComponentConstructor) return;

    // Mount using the physical node as anchor
    const instance = ComponentConstructor(node, {});

    return {
      destroy() {
        if (instance && typeof instance.destroy === 'function') {
          instance.destroy();
        }
      }
    };
  }

  function handleOrderClick(order) {
    selectedOrderId = order.so_id;
    // Hydrate chat with order context
    appState.chatMessages.push({
      role: 'assistant',
      content: `已為您載入 ${order.so_id} (${order.po_reference}) 的上下文資料：\n- 總價：$${order.total_amount.toLocaleString()}\n- 利潤率：${(order.profit_margin * 100).toFixed(0)}%\n- 產能消耗：${(order.capacity_usage * 100).toFixed(0)}%\n\n您可以點擊「核准接單」以觸發安全攔截審查，或向我詢問關於此訂單的排程試算。`
    });
  }

  function triggerAcceptOrder(order) {
    // Intercept action and show confirmation card
    appState.pendingMutation = order;
  }
</script>

{#if appState.route === '/login'}
  <div class="auth-placeholder">
    <!-- TODO: Implement Login view in #28 -->
    <h2>Login Page (Placeholder)</h2>
  </div>
{:else if appState.route === '/register'}
  <div class="auth-placeholder">
    <!-- TODO: Implement Register view in #29 -->
    <h2>Register Page (Placeholder)</h2>
  </div>
{:else if appState.route === '/onboarding'}
  <div class="auth-placeholder">
    <!-- TODO: Implement Onboarding view in #29 -->
    <h2>Onboarding Page (Placeholder)</h2>
  </div>
{:else if appState.route === '/select-tenant'}
  <div class="auth-placeholder">
    <!-- TODO: Implement Select Tenant view in #39 -->
    <h2>Select Tenant Page (Placeholder)</h2>
  </div>
{:else}
  <div class="app-container">
  <!-- Left Sidebar -->
  <aside class="sidebar">
    <div>
      <div class="brand-section">
        <div class="brand-logo">A</div>
        <div style="display: flex; flex-direction: column;">
          <span class="brand-name">AgentERP Edge</span>
          <span style="font-size: 0.75rem; color: var(--text-muted); font-weight: 500;">v{appState.version}</span>
        </div>
      </div>

      <nav>
        <ul class="nav-links">
          <li>
            <div 
              class="nav-item {activeTab === 'sales' ? 'active' : ''}" 
              onclick={() => selectWorkspace('sales')}
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="7" width="20" height="14" rx="2" ry="2"></rect><path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"></path></svg>
              <span>銷售與訂單</span>
            </div>
          </li>
          
          <!-- Dynamic Pluggable Modules -->
          {#each appState.installedModules as mod (mod.id)}
            <li>
              <div 
                class="nav-item {activeTab === mod.id ? 'active' : ''}" 
                onclick={() => selectWorkspace(mod.id)}
              >
                <span class="menu-icon-custom">
                  {@html mod.icon_svg || mod.iconSvg}
                </span>
                <span>{mod.name}</span>
              </div>
            </li>
          {/each}

          <li>
            <div 
              class="nav-item {activeTab === 'settings' ? 'active' : ''}" 
              onclick={() => selectWorkspace('settings')}
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>
              <span>設定與防禦管理</span>
            </div>
          </li>
        </ul>
      </nav>
    </div>

    <div class="sidebar-footer">
      <button class="btn btn-primary" onclick={checkForUpdates}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"></path></svg>
        <span>檢查主程式更新</span>
      </button>
      <div class="connection-ticker">
        <span class="status-dot"></span>
        <span>邊緣資料庫：SQLite 已連線 (v{appState.version})</span>
      </div>
    </div>
  </aside>

  <!-- Main Viewport -->
  <main class="main-viewport">
    <div class="workspace-container">
      
      <!-- Real Cloud Updater Banner (Acceptance Criteria 1: 主頁面下載更新驗收) -->
      {#if appState.updateAvailable}
        <div class="update-banner glass-panel">
          <div class="update-banner-header">
            <span class="badge badge-amber">系統更新可用</span>
            <h4>主程式發現新版本 (v0.2.0)</h4>
          </div>
          <p class="update-notes-preview">{appState.updateNotes}</p>
          
          <div class="update-action-row">
            {#if appState.updateStatus === 'idle'}
              <button class="btn btn-primary" onclick={installUpdate}>
                立刻下載並更新 (Download & Install)
              </button>
            {:else if appState.updateStatus === 'downloading'}
              <div class="update-progress-layout">
                <span class="progress-percent">正在下載：{appState.updateProgress.percent}% ({(appState.updateProgress.downloaded / 1024 / 1024).toFixed(2)} MB / 2.34 MB)</span>
                <div class="progress-bar-container">
                  <div class="progress-bar-fill" style="width: {appState.updateProgress.percent}%"></div>
                </div>
              </div>
            {:else if appState.updateStatus === 'finished'}
              <div class="relaunch-status">
                <span class="spinner-icon"></span>
                <span>下載完成，正在重啟並套用主程式更新...</span>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Page Header -->
      <header class="workspace-header">
        <div class="header-titles">
          <h1>
            {#if activeTab === 'sales'}銷售與訂單管理
            {:else if activeTab === 'settings'}系統與市集管理
            {:else}
              {@const currentMod = appState.installedModules.find(m => m.id === activeTab)}
              {currentMod ? currentMod.name : '企業擴充模組'}
            {/if}
          </h1>
          <p class="subtitle">
            {#if activeTab === 'sales'}管理本期 mirrored orders 與安全確認
            {:else if activeTab === 'settings'}設定 API Key、防禦審計鏈與下載模組市集
            {:else}
              {@const currentMod = appState.installedModules.find(m => m.id === activeTab)}
              {currentMod ? currentMod.description || '動態加載的企業模組介面' : '動態加載的模組頁面'}
            {/if}
          </p>
        </div>

        <div class="header-actions">
          <button class="btn btn-sim-webhook" onclick={triggerWebhookSimulation}>
            <span class="sim-pulse"></span>
            模擬 PO Webhook 送入
          </button>
          
          <div class="notification-trigger" onclick={() => isNotificationOpen = !isNotificationOpen}>
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"></path><path d="M13.73 21a2 2 0 0 1-3.46 0"></path></svg>
            {#if appState.notifications.length > 0}
              <span class="notification-badge">{appState.notifications.length}</span>
            {/if}
          </div>
          
          {#if isNotificationOpen}
            <div class="notification-dropdown glass-panel">
              <div class="notif-header">通知中心 (SSE 管道)</div>
              <div class="notif-list">
                {#if appState.notifications.length === 0}
                  <div class="empty-notif">目前無新訂單或警報。</div>
                {:else}
                  {#each appState.notifications as notif}
                    <div class="notif-item" onclick={() => { navigate('/app/sales'); isNotificationOpen = false; }}>
                      <div class="notif-title">{notif.title}</div>
                      <div class="notif-body">{notif.message}</div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </header>

      <!-- Workspace Contents -->
      <section class="workspace-body">
        
        <!-- 1. SALES WORKSPACE -->
        <div class="workspace-panel" class:hidden={activeTab !== 'sales'}>
          <div class="orders-layout">
            <div class="orders-sidebar">
              <div class="filter-row">
                <button class="filter-btn {activeOrderFilter === 'all' ? 'active' : ''}" onclick={() => activeOrderFilter = 'all'}>全部</button>
                <button class="filter-btn {activeOrderFilter === 'pending' ? 'active' : ''}" onclick={() => activeOrderFilter = 'pending'}>待處理</button>
                <button class="filter-btn {activeOrderFilter === 'approved' ? 'active' : ''}" onclick={() => activeOrderFilter = 'approved'}>已核准</button>
              </div>

              <div class="orders-list">
                {#each appState.mirroredOrders.filter(o => activeOrderFilter === 'all' || o.status === activeOrderFilter) as order}
                  <div 
                    class="order-card {selectedOrderId === order.so_id ? 'selected' : ''}" 
                    onclick={() => handleOrderClick(order)}
                  >
                    <div class="order-card-header">
                      <span class="order-id font-bold">{order.so_id}</span>
                      <span class="badge {order.status === 'approved' ? 'badge-emerald' : 'badge-amber'}">{order.status === 'approved' ? '已核准' : '待審核'}</span>
                    </div>
                    <div class="order-client">{order.customer_name}</div>
                    <div class="order-amount">${order.total_amount.toLocaleString()}</div>
                  </div>
                {/each}
              </div>
            </div>

            <div class="order-detail glass-panel">
              {#if selectedOrderId}
                {@const order = appState.mirroredOrders.find(o => o.so_id === selectedOrderId)}
                {#if order}
                  <div class="detail-header">
                    <h2>銷售訂單草稿：{order.so_id}</h2>
                    <span class="badge {order.status === 'approved' ? 'badge-emerald' : 'badge-amber'}">{order.status === 'approved' ? '已核准放行' : '等候 Peter 安全確認'}</span>
                  </div>
                  
                  <div class="detail-grid">
                    <div class="detail-block">
                      <span class="block-label">客戶採購單參考 (PO Ref):</span>
                      <span class="block-val">{order.po_reference}</span>
                    </div>
                    <div class="detail-block">
                      <span class="block-label">客戶名稱:</span>
                      <span class="block-val">{order.customer_name}</span>
                    </div>
                    <div class="detail-block">
                      <span class="block-label">下單時間:</span>
                      <span class="block-val">{new Date(order.created_at * 1000).toLocaleString()}</span>
                    </div>
                    <div class="detail-block">
                      <span class="block-label">訂單利潤預估:</span>
                      <span class="block-val text-emerald font-bold">{(order.profit_margin * 100).toFixed(0)}%</span>
                    </div>
                  </div>

                  <div class="items-table-container">
                    <h4>訂單明細</h4>
                    <table class="items-table">
                      <thead>
                        <tr>
                          <th>商品名稱</th>
                          <th>數量</th>
                          <th>單價</th>
                          <th>總價</th>
                        </tr>
                      </thead>
                      <tbody>
                        {#each order.items as item}
                          <tr>
                            <td>{item.name}</td>
                            <td>{item.qty}</td>
                            <td>${item.price}</td>
                            <td>${(item.qty * item.price).toLocaleString()}</td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>

                  <!-- Interceptor Action row -->
                  {#if order.status === 'pending'}
                    <div class="ai-copilot-card">
                      <div class="ai-header">
                        <span class="pulse-icon"></span>
                        <h4>AI Agent 預審結果</h4>
                      </div>
                      <p>
                        該採購單利潤率為 25%，消耗邊緣工廠 85% 產能。系統已成功排程生產，剩餘 15% 產能可用於彈性接單。建議 Peter 核准此寫入動作以同步庫存帳本。
                      </p>
                      <button class="btn btn-primary" onclick={() => triggerAcceptOrder(order)}>
                        核准接單並釋放指令
                      </button>
                    </div>
                  {:else}
                    <div class="approved-success-box">
                      <span class="check-icon">✓</span>
                      <div>
                        <h4>訂單已於本地 SQLite 資料庫放行</h4>
                        <p>審計紀錄與加密收據已歸檔，該安全攔截動作已圓滿完成。</p>
                      </div>
                    </div>
                  {/if}
                {/if}
              {:else}
                <div class="detail-placeholder">
                  <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><polyline points="10 9 9 9 8 9"></polyline></svg>
                  <h3>請從左側列表選擇訂單查看詳情</h3>
                  <p>或點擊右上角「模擬 PO Webhook」生成新的訂單鏡像。</p>
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- 2. SETTINGS & AUDIT WORKSPACE -->
        <div class="workspace-panel" class:hidden={activeTab !== 'settings'}>
          <div class="settings-layout">
            <!-- Dynamic Module Marketplace Store Gallery -->
            <div class="settings-group glass-panel">
              <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                <h3>雲端擴充模組市集 (Module Gallery)</h3>
                <button class="btn btn-secondary btn-sm" onclick={fetchModulesGallery} style="padding: 4px 8px; font-size: 0.8rem;">
                  同步商店清單
                </button>
              </div>
              <p class="settings-desc">您可以從雲端清單中點擊選擇下載所需模組。安裝後側邊欄將動態載入對應 SVG 選單且切換時狀態保留。</p>
              
              <div class="module-gallery-list" style="display: grid; grid-template-columns: 1fr; gap: 12px; margin-top: 16px;">
                {#if appState.modulesGallery.length === 0}
                  <div class="empty-audit" style="padding: 20px; text-align: center;">點擊「同步商店清單」載入雲端模組。</div>
                {:else}
                  {#each appState.modulesGallery as item}
                    {@const isInstalled = appState.installedModules.some(m => m.id === item.id)}
                    <div style="display: flex; align-items: flex-start; justify-content: space-between; padding: 12px; border-radius: 8px; border: 1px solid var(--border-color); background: rgba(255, 255, 255, 0.02);">
                      <div style="display: flex; gap: 12px; align-items: center;">
                        <span class="gallery-icon-svg" style="display: flex; align-items: center; justify-content: center; width: 36px; height: 36px; border-radius: 6px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-color);">
                          {@html item.iconSvg}
                        </span>
                        <div>
                          <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="font-weight: 600; color: var(--text-primary);">{item.name}</span>
                            <span style="font-size: 0.75rem; color: var(--text-muted);">v{item.version}</span>
                          </div>
                          <p style="font-size: 0.8rem; color: var(--text-secondary); margin-top: 4px; line-height: 1.3;">{item.description}</p>
                        </div>
                      </div>
                      <div style="display: flex; flex-direction: column; gap: 8px; flex-shrink: 0; align-items: flex-end;">
                        {#if isInstalled}
                          <span style="font-size: 0.8rem; color: var(--text-muted); padding: 4px 8px; border-radius: 4px; background: rgba(255,255,255,0.05); border: 1px solid var(--border-color); font-weight: 500;">已安裝</span>
                          <button 
                            class="btn btn-secondary" 
                            style="padding: 4px 8px; font-size: 0.75rem; border-color: rgba(239, 68, 68, 0.3); color: rgb(239, 68, 68); background: rgba(239, 68, 68, 0.05);"
                            onclick={() => uninstallModuleAction(item.id)}
                          >
                            解除安裝
                          </button>
                        {:else}
                          <button 
                            class="btn btn-primary" 
                            style="padding: 6px 12px; font-size: 0.8rem;"
                            onclick={() => installModuleAction(item.id)}
                          >
                            下載安裝
                          </button>
                        {/if}
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>

            <div class="settings-group glass-panel">
              <h3>邊緣寫入攔截審計紀錄 (SQLite Audit Trail)</h3>
              <p class="settings-desc">記錄每一次經由 Peter 實體點擊確認釋放的 database write 歷史。採用哈希鏈（Hash Chain）防篡改防篡改防篡改防篡改防篡改。</p>
              <div class="audit-list">
                {#if appState.auditLogs.length === 0}
                  <div class="empty-audit">目前尚無寫入審計記錄。</div>
                {:else}
                  {#each appState.auditLogs as log}
                    <div class="audit-item">
                      <div class="audit-meta">
                        <span class="audit-id font-bold">{log.id}</span>
                        <span class="audit-time">{new Date(log.timestamp * 1000).toLocaleString()}</span>
                      </div>
                      <div class="audit-body">
                        <span>指令：<span class="text-amber">{log.action_type}</span></span>
                        <span>操作人：<span class="font-bold">{log.operator}</span></span>
                        <span>結果：<span class={log.decision === 'approved' ? 'text-emerald' : 'text-danger'}>{log.decision === 'approved' ? '核准 (Approved)' : '阻斷 (Rejected)'}</span></span>
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>
          </div>
        </div>

        <!-- 3. Dynamic Pluggable Modules Panels -->
        {#each appState.installedModules as mod (mod.id)}
          {@const Component = appState.loadedComponents[mod.id]}
          <div class="workspace-panel" class:hidden={activeTab !== mod.id}>
            {#if mod.file_path.endsWith('.html')}
              <div class="dynamic-iframe-container glass-panel" style="height: 600px;">
                <iframe 
                  src={isTauri ? `app-module://localhost/modules/${mod.id}_module.html` : `/mock_cdn/${mod.id}_module.html`}
                  class="crm-iframe"
                  title={mod.name}
                  sandbox="allow-scripts"
                  style="width: 100%; height: 100%; border: none; border-radius: 8px;"
                ></iframe>
              </div>
            {:else if Component}
              <div class="glass-panel dynamic-module-container" style="padding: 24px; min-height: 400px; display: flex; flex-direction: column;">
                <div style="margin-bottom: 16px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--border-color); padding-bottom: 12px;">
                  <span class="badge badge-indigo">企業熱插拔模組已載入</span>
                  <span style="font-size: 0.85rem; color: var(--text-muted);">版本 {mod.version} • 安全校驗：SHA-256 OK</span>
                </div>
                <div use:mountModule={mod.id}></div>
              </div>
            {:else}
              <div class="glass-panel" style="padding: 40px; text-align: center;">
                <span style="font-size: 2rem;">⏳</span>
                <h4 style="margin-top: 16px;">正在動態掛載模組元件...</h4>
              </div>
            {/if}
          </div>
        {/each}

      </section>
    </div>

    <!-- Right Chat Box Component -->
    <ChatBox />
  </main>
</div>
{/if}

<!-- Security Confirmation dialog (Mutation Interceptor) -->
<MutationDialog />

<!-- Toast Popup Notification (UX feedback for update checker) -->
{#if appState.toastMessage}
  <div class="toast-popup glass-panel">
    <div class="toast-body">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" style="color: rgb(74, 222, 128);"><polyline points="20 6 9 17 4 12"></polyline></svg>
      <span>{appState.toastMessage}</span>
    </div>
  </div>
{/if}

<style>
  /* Local layout classes */
  .workspace-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--border-color);
    margin-bottom: 24px;
    flex-shrink: 0;
  }

  .subtitle {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin-top: 4px;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 16px;
    position: relative;
  }

  .btn-sim-webhook {
    background: rgba(var(--accent-cyan), 0.08);
    border: 1px solid rgba(var(--accent-cyan), 0.3);
    color: rgb(var(--accent-cyan));
  }

  .btn-sim-webhook:hover {
    background: rgb(var(--accent-cyan));
    color: var(--bg-primary);
  }

  .sim-pulse {
    width: 6px;
    height: 6px;
    background: rgb(var(--accent-cyan));
    border-radius: 50%;
    animation: sim-blink 1.5s infinite;
  }

  @keyframes sim-blink {
    50% { opacity: 0.3; }
  }

  .notification-trigger {
    width: 40px;
    height: 40px;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    position: relative;
    background: var(--bg-secondary);
  }

  .notification-badge {
    position: absolute;
    top: -4px;
    right: -4px;
    background: rgb(var(--accent-amber));
    color: var(--bg-primary);
    font-size: 0.75rem;
    font-weight: 700;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .notification-dropdown {
    position: absolute;
    top: 48px;
    right: 0;
    width: 320px;
    z-index: 100;
    padding: 12px;
  }

  .notif-header {
    font-weight: 600;
    font-size: 0.95rem;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-color);
    margin-bottom: 8px;
  }

  .notif-list {
    max-height: 240px;
    overflow-y: auto;
  }

  .notif-item {
    padding: 10px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .notif-item:hover {
    background: rgba(255, 255, 255, 0.04);
    border-color: var(--border-color);
  }

  .notif-title {
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--text-primary);
  }

  .notif-body {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .empty-notif {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
  }

  /* Sales layout */
  .orders-layout {
    display: flex;
    gap: 24px;
    height: calc(100vh - 180px);
    overflow: hidden;
  }

  .orders-sidebar {
    width: 320px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    flex-shrink: 0;
  }

  .filter-row {
    display: flex;
    gap: 6px;
  }

  .filter-btn {
    flex-grow: 1;
    padding: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
    font-family: inherit;
    font-size: 0.85rem;
    font-weight: 500;
  }

  .filter-btn.active {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border-color: var(--border-active);
  }

  .orders-list {
    flex-grow: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .order-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    padding: 16px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .order-card:hover {
    border-color: var(--border-active);
  }

  .order-card.selected {
    border-color: var(--accent);
    background: rgba(var(--accent-rgb), 0.03);
  }

  .order-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .order-client {
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin-bottom: 4px;
  }

  .order-amount {
    font-size: 1.1rem;
    font-weight: 700;
  }

  .order-detail {
    flex-grow: 1;
    padding: 24px;
    overflow-y: auto;
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border-color);
    margin-bottom: 20px;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
    margin-bottom: 24px;
  }

  .detail-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .block-label {
    color: var(--text-muted);
    font-size: 0.85rem;
  }

  .block-val {
    font-size: 1.05rem;
    color: var(--text-primary);
  }

  .items-table-container {
    margin-bottom: 24px;
  }

  .items-table-container h4 {
    margin-bottom: 10px;
  }

  .items-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.95rem;
  }

  .items-table th, .items-table td {
    padding: 10px 12px;
    text-align: left;
    border-bottom: 1px solid var(--border-color);
  }

  .items-table th {
    color: var(--text-muted);
    font-weight: 500;
  }

  .ai-copilot-card {
    background: rgba(var(--accent-amber), 0.05);
    border: 1px solid rgba(var(--accent-amber), 0.25);
    border-radius: var(--radius-sm);
    padding: 18px;
    margin-top: 10px;
  }

  .ai-header {
    display: flex;
    align-items: center;
    gap: 8px;
    color: rgb(var(--accent-amber));
    margin-bottom: 10px;
  }

  .ai-header h4 {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .ai-copilot-card p {
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.5;
    margin-bottom: 16px;
  }

  .approved-success-box {
    display: flex;
    gap: 16px;
    background: rgba(var(--accent-emerald), 0.05);
    border: 1px solid rgba(var(--accent-emerald), 0.25);
    border-radius: var(--radius-sm);
    padding: 18px;
    margin-top: 10px;
    align-items: center;
  }

  .check-icon {
    font-size: 2.2rem;
    color: rgb(var(--accent-emerald));
    font-weight: 300;
  }

  .approved-success-box h4 {
    color: rgb(var(--accent-emerald));
  }

  .approved-success-box p {
    color: var(--text-secondary);
    font-size: 0.9rem;
    margin-top: 2px;
  }

  .detail-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 12px;
  }

  /* IFrame container */
  .dynamic-iframe-container {
    width: 100%;
    height: calc(100vh - 180px);
    overflow: hidden;
  }

  .crm-iframe {
    width: 100%;
    height: 100%;
    border: none;
    border-radius: var(--radius-md);
  }

  /* Settings Page */
  .settings-layout {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .settings-group {
    padding: 24px;
  }

  .settings-group h3 {
    margin-bottom: 6px;
  }

  .settings-desc {
    color: var(--text-secondary);
    margin-bottom: 16px;
  }

  .settings-status-box {
    margin-bottom: 16px;
  }

  .audit-list {
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 10px;
  }

  .audit-item {
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    padding: 14px;
  }

  .audit-meta {
    display: flex;
    justify-content: space-between;
    margin-bottom: 6px;
    font-size: 0.85rem;
  }

  .audit-time {
    color: var(--text-muted);
  }

  .audit-body {
    display: flex;
    gap: 24px;
    font-size: 0.95rem;
  }

  .text-danger {
    color: #EF4444;
  }

  /* Feature lock screens */
  .feature-locked-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 60px 40px;
    max-width: 480px;
    margin: 40px auto;
  }

  .lock-large {
    font-size: 3rem;
    margin-bottom: 16px;
  }

  .feature-locked-card h2 {
    margin-bottom: 8px;
  }

  .feature-locked-card p {
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 24px;
  }

  .lock-indicator {
    font-size: 0.75rem;
    margin-left: auto;
  }

  /* Updater Banner */
  .update-banner {
    border-color: rgba(var(--accent-amber), 0.3);
    background: rgba(var(--accent-amber), 0.03);
    padding: 16px 20px;
    margin-bottom: 24px;
    flex-shrink: 0;
    animation: slideIn var(--transition-smooth) forwards;
  }

  .update-banner-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }

  .update-notes-preview {
    font-size: 0.9rem;
    color: var(--text-secondary);
    white-space: pre-wrap;
    margin-bottom: 12px;
  }

  .update-action-row {
    display: flex;
    align-items: center;
  }

  .update-progress-layout {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .progress-percent {
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .relaunch-status {
    display: flex;
    align-items: center;
    gap: 10px;
    color: rgb(var(--accent-amber));
    font-weight: 500;
  }

  .spinner-icon {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(var(--accent-amber), 0.3);
    border-top-color: rgb(var(--accent-amber));
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Toast Popup Styles */
  .toast-popup {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 9999;
    padding: 12px 20px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(18, 18, 18, 0.75);
    backdrop-filter: blur(16px);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
    animation: slideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  .toast-body {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 0.9rem;
    color: var(--text-primary);
    font-weight: 500;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }
  .workspace-panel.hidden {
    display: none !important;
  }
  .menu-icon-custom {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    color: inherit;
  }
  .menu-icon-custom :global(svg) {
    width: 100%;
    height: 100%;
    stroke: currentColor;
    fill: none;
  }
  .gallery-icon-svg :global(svg) {
    width: 20px;
    height: 20px;
    stroke: currentColor;
    fill: none;
  }

  /* Auth Screen Placeholder Styles */
  .auth-placeholder {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 100vh;
    width: 100%;
    background: var(--bg-primary);
    color: var(--text-muted);
    font-family: inherit;
    font-size: 1.25rem;
    font-weight: 500;
  }
</style>
