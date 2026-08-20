<script>
  import { selectTenantAction, navigate, appState } from '../../store.svelte.js';

  let isLoading = $state(false);
  let errorMessage = $state('');

  async function handleSelectTenant(tenantId) {
    isLoading = true;
    errorMessage = '';

    try {
      await selectTenantAction(tenantId);
      if (appState.authStatus === 'authenticated') {
        navigate('/app/sales');
      } else {
        errorMessage = '無法切換到選定的租戶，請重新登入';
      }
    } catch (err) {
      console.error("Select tenant failed:", err);
      const code = err?.code || (typeof err === 'string' ? err : '');
      if (code === 'IAM_ERR_TENANT_NOT_ASSIGNED') {
        errorMessage = '無此租戶的存取權限 (IAM_ERR_TENANT_NOT_ASSIGNED)';
      } else {
        errorMessage = `租戶選取失敗：${err?.message || err}`;
      }
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="auth-wrapper">
  <div class="auth-glow"></div>
  <div class="auth-card glass-panel">
    <div class="auth-header">
      <div class="auth-logo">A</div>
      <h2>請選擇要進入的租戶</h2>
      <p class="auth-subtitle">您屬於多個租戶集團，請選擇一個繼續</p>
    </div>

    {#if errorMessage}
      <div class="auth-error-alert">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
        <span>{errorMessage}</span>
      </div>
    {/if}

    <div class="tenants-list">
      {#each appState.authTenants as tenant}
        <button 
          class="tenant-item-btn" 
          disabled={isLoading}
          onclick={() => handleSelectTenant(tenant.id)}
        >
          <div class="tenant-details">
            <span class="tenant-name">{tenant.name}</span>
            <span class="tenant-code">代碼: {tenant.code}</span>
          </div>
          <div class="tenant-role-badge">
            {tenant.role === 'admin' ? '管理者' : '成員'}
          </div>
        </button>
      {/each}
    </div>

    <div class="auth-footer">
      <span>想要登入其他帳號？</span>
      <button class="link-btn" onclick={() => navigate('/login')} disabled={isLoading}>返回登入</button>
    </div>
  </div>
</div>

<style>
  .auth-wrapper {
    position: relative;
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
    background: #0F0F12;
    overflow: hidden;
  }

  .auth-glow {
    position: absolute;
    top: -20%;
    left: 20%;
    width: 60%;
    height: 60%;
    background: radial-gradient(circle, rgba(34, 211, 238, 0.08) 0%, rgba(0, 0, 0, 0) 70%);
    pointer-events: none;
    z-index: 0;
  }

  .auth-card {
    position: relative;
    width: 100%;
    max-width: 440px;
    padding: 40px;
    z-index: 1;
    animation: fade-in-up 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .auth-header {
    text-align: center;
    margin-bottom: 32px;
  }

  .auth-logo {
    width: 48px;
    height: 48px;
    background: linear-gradient(135deg, rgb(34, 211, 238) 0%, rgb(168, 85, 247) 100%);
    color: #0F0F12;
    font-size: 1.5rem;
    font-weight: 700;
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: 12px;
    margin: 0 auto 16px;
    box-shadow: 0 4px 15px rgba(34, 211, 238, 0.3);
  }

  h2 {
    color: var(--text-primary);
    font-size: 1.6rem;
    font-weight: 600;
    margin-bottom: 8px;
  }

  .auth-subtitle {
    color: var(--text-secondary);
    font-size: 0.95rem;
  }

  .tenants-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .tenant-item-btn {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 16px 20px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    transition: all var(--transition-fast);
  }

  .tenant-item-btn:hover:not(:disabled) {
    border-color: rgb(34, 211, 238);
    background: rgba(255, 255, 255, 0.06);
    box-shadow: 0 4px 12px rgba(34, 211, 238, 0.1);
  }

  .tenant-item-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .tenant-details {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tenant-name {
    font-size: 1.05rem;
    font-weight: 600;
  }

  .tenant-code {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .tenant-role-badge {
    padding: 4px 8px;
    font-size: 0.75rem;
    font-weight: 500;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border-color);
    color: var(--text-secondary);
  }

  .auth-error-alert {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: rgb(248, 113, 113);
    padding: 12px 16px;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 20px;
    animation: shake 0.35s ease-in-out;
  }

  .auth-footer {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    margin-top: 28px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-top: 1px solid var(--border-color);
    padding-top: 20px;
  }

  .link-btn {
    background: none;
    border: none;
    color: rgb(34, 211, 238);
    font-weight: 600;
    cursor: pointer;
    padding: 0;
    font-size: 0.85rem;
  }

  .link-btn:hover {
    color: rgb(103, 232, 249);
    text-decoration: underline;
  }

  .link-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  @keyframes fade-in-up {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-4px); }
    75% { transform: translateX(4px); }
  }
</style>
