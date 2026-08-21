<script>
  import { appState, approveMutation, rejectMutation } from '../store.svelte.js';
</script>

{#if appState.pendingMutation}
  <div class="security-overlay">
    <div class="security-card">
      <div class="security-title-row">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>
        <span>邊緣寫入攔截器 (Mutation Interceptor)</span>
      </div>
      
      <p class="security-warning">
        偵測到 AI Agent 試圖修改本地 ERP 核心數據庫。該防禦動作已被安全扣留，需要 {appState.authUser?.display_name || appState.authUser?.email || '操作者'} 進行實體審核確認。
      </p>

      <div class="security-info-box">
        <div class="info-row">
          <span class="info-label">操作指令:</span>
          <span class="info-value text-amber">db_ledger::confirm_mirrored_order</span>
        </div>
        <div class="info-row">
          <span class="info-label">對象訂單:</span>
          <span class="info-value">{appState.pendingMutation.id} ({appState.pendingMutation.po_reference})</span>
        </div>
        <div class="info-row">
          <span class="info-label">客戶對象:</span>
          <span class="info-value">{appState.pendingMutation.customer_name}</span>
        </div>
        <div class="info-row">
          <span class="info-label">合約總額:</span>
          <span class="info-value font-bold">${appState.pendingMutation.total_amount.toLocaleString()}</span>
        </div>
        <div class="info-row">
          <span class="info-label">預估利潤率:</span>
          <span class="info-value text-emerald font-bold">{(appState.pendingMutation.profit_margin * 100).toFixed(0)}%</span>
        </div>
        <div class="info-row">
          <span class="info-label">產能預排比:</span>
          <span class="info-value font-bold">{(appState.pendingMutation.capacity_usage * 100).toFixed(0)}% (剩餘 15% 產能)</span>
        </div>
      </div>

      <div class="action-buttons">
        <button 
          class="btn btn-reject" 
          onclick={() => rejectMutation(appState.pendingMutation.id)}
        >
          安全阻斷 (Reject)
        </button>
        <button 
          class="btn btn-approve" 
          onclick={() => approveMutation(appState.pendingMutation.id)}
        >
          授權放行 (Approve)
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .security-warning {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin-bottom: 20px;
    line-height: 1.5;
    text-align: left;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    padding: 8px 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    font-size: 0.95rem;
  }

  .info-row:last-child {
    border-bottom: none;
  }

  .info-label {
    color: var(--text-muted);
  }

  .info-value {
    color: var(--text-primary);
  }

  .text-amber {
    color: rgb(var(--accent-amber));
    font-family: monospace;
    font-weight: 600;
  }

  .text-emerald {
    color: rgb(var(--accent-emerald));
  }

  .font-bold {
    font-weight: 600;
  }

  .action-buttons {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 10px;
  }

  .btn {
    padding: 10px 22px;
    border-radius: var(--radius-sm);
    font-weight: 600;
    font-family: inherit;
    font-size: 0.95rem;
    cursor: pointer;
    border: none;
    transition: background var(--transition-fast);
  }

  .btn-reject {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    color: var(--text-primary);
  }

  .btn-reject:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .btn-approve {
    background: rgb(var(--accent-amber));
    color: var(--bg-primary);
    box-shadow: 0 0 16px rgba(var(--accent-amber), 0.2);
  }

  .btn-approve:hover {
    background: rgba(var(--accent-amber), 0.85);
  }
</style>
