<script>
  import { createTenantAction, navigate, appState } from '../../store.svelte.js';

  let tenantCode = $state('');
  let tenantName = $state('');
  let companyName = $state('');
  let taxId = $state('');
  
  let isLoading = $state(false);
  let errorMessage = $state('');
  
  // Inline validation errors
  let errors = $state({
    tenantCode: '',
    tenantName: '',
    companyName: ''
  });

  function validateForm() {
    let isValid = true;
    errors.tenantCode = '';
    errors.tenantName = '';
    errors.companyName = '';

    if (!tenantCode.trim()) {
      errors.tenantCode = '請輸入集團代碼';
      isValid = false;
    } else if (!/^[a-zA-Z0-9_-]+$/.test(tenantCode)) {
      errors.tenantCode = '集團代碼只能包含英文字母、數字、底線與連字號';
      isValid = false;
    }

    if (!tenantName.trim()) {
      errors.tenantName = '請輸入集團名稱';
      isValid = false;
    }

    if (!companyName.trim()) {
      errors.companyName = '請輸入公司名稱';
      isValid = false;
    }

    return isValid;
  }

  async function handleSubmit(e) {
    e.preventDefault();
    if (!validateForm()) return;

    isLoading = true;
    errorMessage = '';

    try {
      await createTenantAction(tenantName, companyName, tenantCode, taxId);
      if (appState.authStatus === 'authenticated') {
        navigate('/app/sales');
      } else {
        errorMessage = '建立成功但狀態未同步，請重新登入';
      }
    } catch (err) {
      console.error("Create tenant failed:", err);
      if (err === 'IAM_ERR_TENANT_CODE_TAKEN') {
        errors.tenantCode = '此集團代碼已被使用 (IAM_ERR_TENANT_CODE_TAKEN)';
      } else {
        errorMessage = `建立公司失敗：${err}`;
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
      <h2>建立新集團與公司</h2>
      <p class="auth-subtitle">這是您第一次登入，請先完成初始化引導</p>
    </div>

    {#if errorMessage}
      <div class="auth-error-alert">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
        <span>{errorMessage}</span>
      </div>
    {/if}

    <form onsubmit={handleSubmit} class="auth-form">
      <div class="form-group">
        <label for="tenantCode">集團代碼 (Group Code) <span class="required">*</span></label>
        <input 
          type="text" 
          id="tenantCode" 
          bind:value={tenantCode}
          placeholder="例如: numax-group"
          disabled={isLoading}
          required
        />
        {#if errors.tenantCode}
          <span class="field-error">{errors.tenantCode}</span>
        {/if}
      </div>

      <div class="form-group">
        <label for="tenantName">集團名稱 (Group Name) <span class="required">*</span></label>
        <input 
          type="text" 
          id="tenantName" 
          bind:value={tenantName}
          placeholder="例如: 紐碼科技集團"
          disabled={isLoading}
          required
        />
        {#if errors.tenantName}
          <span class="field-error">{errors.tenantName}</span>
        {/if}
      </div>

      <div class="form-group">
        <label for="companyName">第一家子公司名稱 (Company Name) <span class="required">*</span></label>
        <input 
          type="text" 
          id="companyName" 
          bind:value={companyName}
          placeholder="例如: 紐碼科技股份有限公司"
          disabled={isLoading}
          required
        />
        {#if errors.companyName}
          <span class="field-error">{errors.companyName}</span>
        {/if}
      </div>

      <div class="form-group">
        <label for="taxId">統一編號 (Tax ID) <span class="optional">(選填)</span></label>
        <input 
          type="text" 
          id="taxId" 
          bind:value={taxId}
          placeholder="例如: 12345678"
          disabled={isLoading}
        />
      </div>

      <button type="submit" class="btn btn-primary auth-submit-btn" disabled={isLoading}>
        {#if isLoading}
          <span class="spinner"></span> 正在建立與初始化...
        {:else}
          完成建立並進入工作區
        {/if}
      </button>
    </form>

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
    max-width: 460px;
    padding: 40px;
    z-index: 1;
    animation: fade-in-up 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .auth-header {
    text-align: center;
    margin-bottom: 24px;
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
    gap: 16px;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 0.85rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .required {
    color: #EF4444;
    margin-left: 2px;
  }

  .optional {
    color: var(--text-muted);
    font-size: 0.75rem;
    margin-left: 4px;
  }

  input {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-color);
    padding: 10px 14px;
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

  input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .field-error {
    color: #F87171;
    font-size: 0.8rem;
    margin-top: 4px;
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
    margin-top: 12px;
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
    margin-bottom: 16px;
    animation: shake 0.35s ease-in-out;
  }

  .auth-footer {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    margin-top: 24px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-top: 1px solid var(--border-color);
    padding-top: 16px;
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
