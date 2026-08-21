<script>
  import { login, navigate, appState } from '../../store.svelte.js';

  let email = $state('');
  let password = $state('');
  let isLoading = $state(false);
  let errorMessage = $state('');

  async function handleLogin(e) {
    e.preventDefault();
    if (!email || !password) {
      errorMessage = '請輸入電子郵件與密碼';
      return;
    }

    isLoading = true;
    errorMessage = '';

    try {
      await login(email, password);
      // Determine where to navigate based on new auth status
      if (appState.authStatus === 'authenticated') {
        navigate('/app/sales');
      } else if (appState.authStatus === 'needs_tenant_creation') {
        navigate('/onboarding');
      } else if (appState.authStatus === 'needs_tenant_selection') {
        navigate('/select-tenant');
      }
    } catch (err) {
      console.error("Login failed:", err);
      const code = err?.code || err;
      if (code === 'IAM_ERR_INVALID_CREDENTIALS') {
        errorMessage = '帳號或密碼錯誤 (IAM_ERR_INVALID_CREDENTIALS)';
      } else {
        errorMessage = `登入失敗：${err?.message || err?.code || err}`;
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
      <h2>歡迎回到 AgentERP</h2>
      <p class="auth-subtitle">請登入您的邊緣安全工作站帳戶</p>
    </div>

    {#if errorMessage}
      <div class="auth-error-alert">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
        <span>{errorMessage}</span>
      </div>
    {/if}

    <form onsubmit={handleLogin} class="auth-form">
      <div class="form-group">
        <label for="email">電子郵件 (Email)</label>
        <input 
          type="email" 
          id="email" 
          bind:value={email}
          placeholder="admin@example.com"
          disabled={isLoading}
          required
        />
      </div>

      <div class="form-group">
        <div class="label-row">
          <label for="password">密碼 (Password)</label>
        </div>
        <input 
          type="password" 
          id="password" 
          bind:value={password}
          placeholder="••••••••"
          disabled={isLoading}
          required
        />
      </div>

      <button type="submit" class="btn btn-primary auth-submit-btn" disabled={isLoading}>
        {#if isLoading}
          <span class="spinner"></span> 登入中...
        {:else}
          安全登入
        {/if}
      </button>
    </form>

    <div class="auth-footer">
      <span>尚未建立公司與帳號？</span>
      <button class="link-btn" onclick={() => navigate('/register')}>註冊新帳號</button>
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

  .auth-form {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  label {
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  input {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    padding: 12px 16px;
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.95rem;
    transition: all var(--transition-fast);
  }

  input:focus {
    outline: none;
    border-color: rgb(34, 211, 238);
    background: rgba(255, 255, 255, 0.06);
    box-shadow: 0 0 0 3px rgba(34, 211, 238, 0.15);
  }

  .btn {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 10px;
    padding: 12px 24px;
    border-radius: var(--radius-sm);
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-primary {
    background: rgb(34, 211, 238);
    color: #0F0F12;
    border: none;
  }

  .btn-primary:hover:not(:disabled) {
    background: rgb(103, 232, 249);
    box-shadow: 0 4px 12px rgba(34, 211, 238, 0.25);
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .auth-submit-btn {
    margin-top: 8px;
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

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(15, 15, 18, 0.25);
    border-top: 2px solid #0F0F12;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
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
