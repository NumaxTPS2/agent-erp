<script>
  import { registerTenant, navigate, appState } from '../../store.svelte.js';

  let currentStep = $state(1); // 1: Account, 2: Company
  
  // Form fields
  let adminEmail = $state('');
  let adminPassword = $state('');
  let confirmPassword = $state('');
  let tenantName = $state('');
  let companyName = $state('');
  let tenantCode = $state('');

  // UI state
  let isLoading = $state(false);
  let errorMessage = $state('');

  // Password strength calculations
  let passwordStrength = $derived.by(() => {
    if (!adminPassword) return { score: 0, label: '無', color: 'gray' };
    let score = 0;
    if (adminPassword.length >= 8) score += 1;
    if (/[A-Z]/.test(adminPassword)) score += 1;
    if (/[0-9]/.test(adminPassword)) score += 1;
    if (/[^A-Za-z0-9]/.test(adminPassword)) score += 1;

    if (score <= 1) return { score, label: '弱 (太短或太簡單)', color: 'rgb(239, 68, 68)' };
    if (score === 2) return { score, label: '中等', color: 'rgb(245, 158, 11)' };
    if (score >= 3) return { score, label: '強', color: 'rgb(16, 185, 129)' };
  });

  // Step 1 validation
  let isStep1Valid = $derived(
    adminEmail && 
    adminPassword && 
    adminPassword.length >= 8 && 
    adminPassword === confirmPassword
  );

  // Step 2 validation
  let isStep2Valid = $derived(
    tenantName && 
    companyName && 
    tenantCode && 
    /^[a-z0-9-_]+$/.test(tenantCode)
  );

  function nextStep() {
    if (currentStep === 1 && isStep1Valid) {
      errorMessage = '';
      currentStep = 2;
    }
  }

  function prevStep() {
    if (currentStep === 2) {
      currentStep = 1;
    }
  }

  async function handleSubmit(e) {
    e.preventDefault();
    if (!isStep1Valid || !isStep2Valid) {
      errorMessage = '請填寫所有必要欄位且確認格式正確';
      return;
    }

    isLoading = true;
    errorMessage = '';

    try {
      await registerTenant(tenantName, companyName, adminEmail, adminPassword, tenantCode);
      if (appState.authStatus === 'authenticated') {
        navigate('/app/sales');
      } else if (appState.authStatus === 'needs_tenant_creation') {
        navigate('/onboarding');
      } else if (appState.authStatus === 'needs_tenant_selection') {
        navigate('/select-tenant');
      }
    } catch (err) {
      console.error("Registration failed:", err);
      if (err === 'IAM_ERR_EMAIL_TAKEN') {
        errorMessage = '該電子郵件已被註冊過 (IAM_ERR_EMAIL_TAKEN)';
        currentStep = 1; // Send back to step 1
      } else if (err === 'IAM_ERR_WEAK_PASSWORD') {
        errorMessage = '密碼強度不足，請使用更複雜的密碼 (IAM_ERR_WEAK_PASSWORD)';
        currentStep = 1;
      } else {
        errorMessage = `註冊失敗：${err}`;
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
      <h2>註冊新帳號與公司</h2>
      <p class="auth-subtitle">開始您的 AgentERP 安全自動化管理</p>
    </div>

    <!-- Stepper indicator -->
    <div class="stepper">
      <div class="step-indicator {currentStep >= 1 ? 'active' : ''}">
        <span class="step-num">1</span>
        <span class="step-text">帳號設定</span>
      </div>
      <div class="step-line {currentStep >= 2 ? 'active' : ''}"></div>
      <div class="step-indicator {currentStep >= 2 ? 'active' : ''}">
        <span class="step-num">2</span>
        <span class="step-text">公司租戶</span>
      </div>
    </div>

    {#if errorMessage}
      <div class="auth-error-alert">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
        <span>{errorMessage}</span>
      </div>
    {/if}

    <form onsubmit={handleSubmit} class="auth-form">
      {#if currentStep === 1}
        <!-- STEP 1: ACCOUNT DETAILS -->
        <div class="step-panel">
          <div class="form-group">
            <label for="adminEmail">管理員電子郵件 (Admin Email)</label>
            <input 
              type="email" 
              id="adminEmail" 
              bind:value={adminEmail}
              placeholder="admin@example.com"
              disabled={isLoading}
              required
            />
          </div>

          <div class="form-group">
            <label for="adminPassword">管理員密碼 (Password)</label>
            <input 
              type="password" 
              id="adminPassword" 
              bind:value={adminPassword}
              placeholder="至少 8 位字元"
              disabled={isLoading}
              required
            />
            {#if adminPassword}
              <div class="password-meter-container">
                <div class="password-meter-bar" style="background: {passwordStrength.color}; width: {passwordStrength.score * 25}%"></div>
                <span class="password-meter-label" style="color: {passwordStrength.color}">{passwordStrength.label}</span>
              </div>
            {/if}
          </div>

          <div class="form-group">
            <label for="confirmPassword">確認密碼 (Confirm Password)</label>
            <input 
              type="password" 
              id="confirmPassword" 
              bind:value={confirmPassword}
              placeholder="再次輸入密碼"
              disabled={isLoading}
              required
            />
            {#if confirmPassword && adminPassword !== confirmPassword}
              <span class="field-error">密碼輸入不一致</span>
            {/if}
          </div>

          <button 
            type="button" 
            class="btn btn-primary step-btn" 
            onclick={nextStep}
            disabled={!isStep1Valid}
          >
            下一步：填寫公司資料
          </button>
        </div>
      {:else if currentStep === 2}
        <!-- STEP 2: COMPANY/TENANT DETAILS -->
        <div class="step-panel">
          <div class="form-group">
            <label for="tenantName">集團租戶名稱 (Tenant Group Name)</label>
            <input 
              type="text" 
              id="tenantName" 
              bind:value={tenantName}
              placeholder="e.g. 努瑪斯科技集團"
              disabled={isLoading}
              required
            />
          </div>

          <div class="form-group">
            <label for="companyName">公司法定名稱 (Company Legal Name)</label>
            <input 
              type="text" 
              id="companyName" 
              bind:value={companyName}
              placeholder="e.g. 努瑪斯股份有限公司"
              disabled={isLoading}
              required
            />
          </div>

          <div class="form-group">
            <label for="tenantCode">租戶代碼 (Tenant Code / Slug)</label>
            <input 
              type="text" 
              id="tenantCode" 
              bind:value={tenantCode}
              placeholder="e.g. numax-tech (限小寫英文、數字與底線/減號)"
              disabled={isLoading}
              required
            />
            {#if tenantCode && !/^[a-z0-9-_]+$/.test(tenantCode)}
              <span class="field-error">代碼格式不正確，僅限小寫英文、數字與 - _</span>
            {/if}
          </div>

          <div class="button-row">
            <button 
              type="button" 
              class="btn btn-secondary" 
              onclick={prevStep}
              disabled={isLoading}
            >
              返回
            </button>
            <button 
              type="submit" 
              class="btn btn-primary step-btn" 
              disabled={!isStep2Valid || isLoading}
            >
              {#if isLoading}
                <span class="spinner"></span> 創建中...
              {:else}
                完成並建立公司
              {/if}
            </button>
          </div>
        </div>
      {/if}
    </form>

    <div class="auth-footer">
      <span>已經有帳號了？</span>
      <button class="link-btn" onclick={() => navigate('/login')}>登入帳號</button>
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
    max-width: 480px;
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

  /* Stepper indicator */
  .stepper {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 32px;
    padding: 0 16px;
  }

  .step-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    opacity: 0.35;
    transition: all var(--transition-smooth);
  }

  .step-indicator.active {
    opacity: 1;
  }

  .step-num {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid var(--text-secondary);
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-weight: 600;
    display: flex;
    justify-content: center;
    align-items: center;
    transition: all var(--transition-fast);
  }

  .step-indicator.active .step-num {
    background: rgb(34, 211, 238);
    border-color: rgb(34, 211, 238);
    color: #0F0F12;
    box-shadow: 0 0 10px rgba(34, 211, 238, 0.4);
  }

  .step-text {
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .step-indicator.active .step-text {
    color: var(--text-primary);
  }

  .step-line {
    flex-grow: 1;
    height: 2px;
    background: var(--border-color);
    margin: 0 16px;
    margin-bottom: 24px;
    transition: all var(--transition-smooth);
  }

  .step-line.active {
    background: rgb(34, 211, 238);
  }

  .auth-form {
    display: flex;
    flex-direction: column;
  }

  .step-panel {
    display: flex;
    flex-direction: column;
    gap: 20px;
    animation: slide-in 0.35s cubic-bezier(0.4, 0, 0.2, 1);
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

  .field-error {
    font-size: 0.78rem;
    color: rgb(248, 113, 113);
    margin-top: -2px;
  }

  .password-meter-container {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 4px;
  }

  .password-meter-bar {
    height: 4px;
    border-radius: 2px;
    flex-grow: 1;
    max-width: 120px;
    background: #374151;
    transition: all var(--transition-fast);
  }

  .password-meter-label {
    font-size: 0.78rem;
    font-weight: 500;
  }

  .button-row {
    display: flex;
    gap: 16px;
    margin-top: 8px;
  }

  .button-row .btn {
    flex-grow: 1;
  }

  .step-btn {
    width: 100%;
    margin-top: 8px;
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
    opacity: 0.55;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: transparent;
    border: 1px solid var(--border-color);
    color: var(--text-primary);
  }

  .btn-secondary:hover:not(:disabled) {
    border-color: var(--border-active);
    background: rgba(255, 255, 255, 0.04);
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
    margin-bottom: 24px;
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

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateX(10px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-4px); }
    75% { transform: translateX(4px); }
  }
</style>
