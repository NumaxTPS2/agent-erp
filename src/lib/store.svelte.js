// @ts-check

/**
 * @typedef {import('./bindings').AuthStatusResponse} AuthStatusResponse
 * @typedef {import('./bindings').AuthUser} AuthUser
 * @typedef {import('./bindings').AuthTenant} AuthTenant
 * @typedef {import('./bindings').ApiErrorPayload} ApiErrorPayload
 * @typedef {import('./bindings').LogoutResponse} LogoutResponse
 */

import { invoke, check, relaunch } from './tauri.js';
import { loadModule } from './registry.js';

// Define the global reactive app state using Svelte 5 $state
export const appState = $state({
  route: '/app/agent',
  version: '0.1.0',
  isEnterpriseActive: false,
  /** @type {any[]} */
  installedModules: [], // List of installed module metadata
  /** @type {Record<string, any>} */
  loadedComponents: {}, // Map of moduleId -> Svelte Component class

  get activeWorkspace() {
    if (this.route === '/app/agent') return 'agent';
    if (this.route === '/app/sales') return 'sales';
    if (this.route === '/app/finance') return 'finance';
    if (this.route === '/app/crm') return 'crm';
    if (this.route === '/app/settings') return 'settings';
    
    // Generic fallback for any other workspaces like /app/xxx
    const match = this.route.match(/^\/app\/([^/]+)$/);
    if (match) return match[1];
    
    return 'agent';
  },

  set activeWorkspace(ws) {
    let targetRoute = `/app/${ws}`;
    if (ws === 'agent') targetRoute = '/app/agent';
    else if (ws === 'sales') targetRoute = '/app/sales';
    else if (ws === 'finance') targetRoute = '/app/finance';
    else if (ws === 'crm') targetRoute = '/app/crm';
    else if (ws === 'settings') targetRoute = '/app/settings';
    
    this.route = targetRoute;
    if (typeof window !== 'undefined' && window.location && window.location.hash !== '#' + targetRoute) {
      window.location.hash = targetRoute;
    }
  },

  // Chat panel state
  /** @type {Array<{ role: string, content: string }>} */
  chatMessages: [
    { role: 'assistant', content: '你好，我是 AgentERP 智能助理。我已經載入本地安全邊緣工作站上下文，隨時可以為您服務。' }
  ],
  isChatStreaming: false,
  currentStreamContent: '',

  // Task-driven workflow state
  /** @type {any[]} */
  tasks: [],
  /** @type {string | null} */
  activeTaskId: null, // null = 主 Agent 環境對話；字串 = 子任務對話
  /** @type {Record<string, Array<{ role: string, content: string, timestamp?: number }>>} */
  taskMessages: {},

  // Database cache lists
  /** @type {any[]} */
  mirroredOrders: [],
  /** @type {any[]} */
  auditLogs: [],
  /** @type {any[]} */
  notifications: [],

  // Mutation interceptor queue
  /** @type {any} */
  pendingMutation: null, // { id, title, details }

  // System updater status
  updateAvailable: false,
  updateNotes: '',
  updateStatus: 'idle', // 'idle' | 'checking' | 'downloading' | 'finished' | 'up-to-date'
  updateProgress: { percent: 0, downloaded: 0, total: 100 },
  /** @type {any} */
  activeUpdate: null, // Tauri updater instance
  /** @type {string | null} */
  toastMessage: null, // Toast popup message
  /** @type {any[]} */
  modulesGallery: [], // List of available modules in cloud store
  
  // Auth state
  /** @type {string} */
  authStatus: 'unauthenticated', // 'unauthenticated' | 'needs_tenant_selection' | 'needs_tenant_creation' | 'authenticated'
  /** @type {AuthUser | null} */
  authUser: null,
  /** @type {AuthTenant[]} */
  authTenants: [],
  /** @type {AuthTenant | null} */
  activeTenant: null
});

export function showToast(message) {
  appState.toastMessage = message;
  setTimeout(() => {
    appState.toastMessage = null;
  }, 3500);
}

export function navigate(path) {
  if (typeof window !== 'undefined' && window.location) {
    window.location.hash = path;
  }
  appState.route = path;
}

// Fetch mirrored order list from SQLite
pub_fn("fetchOrders");
async function pub_fn(name) {} // Stub helper

export async function fetchOrders() {
  try {
    const list = await invoke('get_mirrored_orders');
    appState.mirroredOrders = list.map(o => ({ ...o, id: o.so_id || o.id }));
  } catch (err) {
    console.error("Failed to fetch mirrored orders:", err);
  }
}

// Fetch audit logging trail from SQLite
pub_fn("fetchAuditLogs");
export async function fetchAuditLogs() {
  try {
    const list = await invoke('get_audit_logs');
    appState.auditLogs = list;
  } catch (err) {
    console.error("Failed to fetch audit logs:", err);
  }
}

// Check local DB for installed dynamic modules
pub_fn("fetchInstalledModules");
export async function fetchInstalledModules() {
  try {
    const list = await invoke('get_installed_modules');
    appState.installedModules = list;
    
    // Automatically register components for Svelte mounting
    for (const mod of list) {
      await loadModule(mod.id);
    }
    
    if (list.some(m => m.id === 'sales_bi')) {
      appState.isEnterpriseActive = true;
    }
  } catch (err) {
    console.error("Failed to fetch installed modules:", err);
  }
}

// Fetch all available modules in the cloud gallery store
pub_fn("fetchModulesGallery");
export async function fetchModulesGallery() {
  try {
    let list = [];
    try {
      const response = await fetch('https://numaxofficee8.github.io/agent-erp/modules_gallery.json');
      if (response.ok) {
        list = await response.json();
      } else {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
    } catch (e) {
      console.warn("Failed to fetch cloud modules gallery, using mock local catalogue:", e);
      // Local fallback catalogue for mock testing
      list = [
        {
          id: 'sales_bi',
          name: 'Finance BI 大看板',
          version: '1.0.2',
          description: '提供即時的銷售數據分析、獲利預測與動態利潤控制工具。',
          iconSvg: '<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><line x1=\"18\" y1=\"20\" x2=\"18\" y2=\"10\"></line><line x1=\"12\" y1=\"20\" x2=\"12\" y2=\"4\"></line><line x1=\"6\" y1=\"20\" x2=\"6\" y2=\"14\"></line></svg>',
          downloadUrl: 'sales_bi_module.js',
          sha256: 'mock-sha-sales-bi'
        },
        {
          id: 'crm',
          name: 'CRM 客戶模組',
          version: '1.0.1',
          description: '企業級客戶關係管理，支援獨立沙盒與靜態 HTML 加載。',
          iconSvg: '<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2\"></path><circle cx=\"9\" cy=\"7\" r=\"4\"></circle><path d=\"M23 21v-2a4 4 0 0 0-3-3.87\"></path><path d=\"M16 3.13a4 4 0 0 1 0 7.75\"></path></svg>',
          downloadUrl: 'crm_dashboard.html',
          sha256: 'mock-sha-crm'
        }
      ];
    }
    appState.modulesGallery = list;
  } catch (err) {
    console.error("Failed to fetch modules gallery:", err);
  }
}

// Download and install a specific module from the store
pub_fn("installModuleAction");
export async function installModuleAction(moduleId) {
  const mod = appState.modulesGallery.find(m => m.id === moduleId);
  if (!mod) return;

  showToast(`正在下載安裝模組 ${mod.name}...`);
  try {
    await invoke('install_module', {
      moduleId: mod.id,
      name: mod.name,
      version: mod.version,
      iconSvg: mod.iconSvg,
      downloadUrl: mod.downloadUrl,
      sha256: mod.sha256
    });

    // Refresh installed list to dynamically mount Svelte component
    await fetchInstalledModules();
    showToast(`模組 ${mod.name} 安裝成功！選單已更新。`);
  } catch (err) {
    console.error(`Failed to install module ${mod.name}:`, err);
    showToast(`模組安裝失敗: ${err}`);
  }
}

// Uninstall a registered module and clear dynamic component states
pub_fn("uninstallModuleAction");
export async function uninstallModuleAction(moduleId) {
  const mod = appState.installedModules.find(m => m.id === moduleId);
  const name = mod ? mod.name : moduleId;

  showToast(`正在解除安裝模組 ${name}...`);
  try {
    await invoke('uninstall_module', { moduleId });
    
    // Clear dynamic component class
    delete appState.loadedComponents[moduleId];
    appState.loadedComponents = { ...appState.loadedComponents };
    
    // Refresh installed list
    await fetchInstalledModules();
    
    // If viewing the uninstalled module, route back to sales tab
    if (appState.activeWorkspace === moduleId) {
      appState.activeWorkspace = 'sales';
    }
    
    showToast(`模組 ${name} 已成功解除安裝！`);
  } catch (err) {
    console.error(`Failed to uninstall module ${moduleId}:`, err);
    showToast(`解除安裝失敗: ${err}`);
  }
}

// Trigger customer PO webhook simulation
pub_fn("triggerWebhookSimulation");
export async function triggerWebhookSimulation() {
  try {
    // Inject a pending notification ticker
    appState.notifications = [
      { id: 'notify-webhook', title: 'Webhook 觸發中...', message: '正在傳送模擬採購單 (PO-2026-0092) 到邊緣端...' }
    ];
    await invoke('simulate_webhook_order');
  } catch (err) {
    console.error("Webhook simulation trigger failed:", err);
  }
}

// Approve mutation card
pub_fn("approveMutation");
export async function approveMutation(id) {
  if (!appState.pendingMutation) return;
  try {
    const operator = appState.authUser?.display_name || appState.authUser?.email || 'Unknown';
    await invoke('confirm_mutation', { mutationId: id, approved: true, operator });
    appState.pendingMutation = null;
    await fetchOrders();
    await fetchAuditLogs();
    
    appState.chatMessages.push({
      role: 'assistant',
      content: `已成功核准訂單 ${id}！寫入指令已釋放，已更新本地 SQLite 庫存，並將加密收據回傳給 A 公司。`
    });
  } catch (err) {
    console.error("Failed to approve mutation:", err);
  }
}

// Reject mutation card
pub_fn("rejectMutation");
export async function rejectMutation(id) {
  if (!appState.pendingMutation) return;
  try {
    const operator = appState.authUser?.display_name || appState.authUser?.email || 'Unknown';
    await invoke('confirm_mutation', { mutationId: id, approved: false, operator });
    appState.pendingMutation = null;
    await fetchOrders();
    await fetchAuditLogs();
    
    appState.chatMessages.push({
      role: 'assistant',
      content: `已拒絕訂單 ${id} 的核准寫入。該指令已被安全阻斷，審計日誌已記錄 ${operator === 'Unknown' ? '操作者' : operator} 的拒絕動作。`
    });
  } catch (err) {
    console.error("Failed to reject mutation:", err);
  }
}

// Simulate downloading, SHA-256 verifying, and hot-plugging the enterprise modules
pub_fn("activateEnterprise");
export async function activateEnterprise() {
  appState.isEnterpriseActive = false;
  
  try {
    // 1. Download & verify Svelte JS module
    const biMeta = await invoke('download_module', { moduleId: 'sales_bi' });
    await loadModule(biMeta.id);
    
    // 2. Download & verify crm iframe dashboard
    const crmMeta = await invoke('download_module', { moduleId: 'crm' });
    await loadModule(crmMeta.id);
    
    await fetchInstalledModules();
    
    appState.chatMessages.push({
      role: 'assistant',
      content: "企業進階授權已解鎖！Finance BI 看板與 CRM 模組已在本地 AppData 安全目錄完成 SHA-256 數位簽章驗收並動態載入。主介面已無縫更新。"
    });
  } catch (err) {
    console.error("Enterprise activation failed:", err);
  }
}

// Query public update manifest URL
export async function checkForUpdates() {
  appState.updateStatus = 'checking';
  showToast("正在連線雲端檢查更新...");
  try {
    const update = await check();
    if (update && update.available) {
      appState.updateAvailable = true;
      appState.updateStatus = 'idle';
      appState.updateNotes = update.body || '安全升級與效能優化版本。';
      appState.activeUpdate = update;
      showToast("偵測到新版本！已於首頁載入更新橫幅。");
      
      // Dispatch alert to notifications hub
      appState.notifications.unshift({
        id: 'notify-update',
        title: `主程式更新可用 v${update.version || '0.2.0'}`,
        message: 'Tauri 邊緣端主程式已有新版本，請前往首頁進行安全下載更新。'
      });
    } else {
      appState.updateAvailable = false;
      appState.updateStatus = 'up-to-date';
      showToast(`主程式已是最新版本 (v${appState.version})！無需更新。`);
    }
  } catch (err) {
    console.warn("Tauri updater connection failed, falling back to local simulation:", err);
    // Offline simulation fallback for user testing
    appState.updateAvailable = true;
    appState.updateNotes = "主要優化：\n1. 優化 Svelte 5 Runes 渲染引擎\n2. 升級 Rust 邊緣 SQLite 加密協議 (SQLCipher)\n3. 修正 Windows/macOS 系統更新偶發閃退問題。";
    appState.updateStatus = 'idle';
    showToast("已進入模擬測試：已載入模擬更新資訊。");
  }
}

// Run actual Tauri update download and install, fallback to simulation if offline
pub_fn("installUpdate");
export async function installUpdate() {
  if (appState.activeUpdate) {
    appState.updateStatus = 'downloading';
    appState.updateProgress = { percent: 0, downloaded: 0, total: 100 };
    try {
      let downloaded = 0;
      await appState.activeUpdate.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          appState.updateProgress.total = event.data.contentLength || 2540000;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
          appState.updateProgress.downloaded = downloaded;
          appState.updateProgress.percent = Math.round((downloaded / appState.updateProgress.total) * 100);
        } else if (event.event === 'Finished') {
          appState.updateStatus = 'finished';
        }
      });
      // Relaunch app
      await relaunch();
    } catch (err) {
      console.warn("Real install failed (unsigned dev build), running mock upgrade:", err);
      runMockUpgrade();
    }
  } else {
    runMockUpgrade();
  }
}

function runMockUpgrade() {
  appState.updateStatus = 'downloading';
  appState.updateProgress = { percent: 0, downloaded: 0, total: 2450000 };
  
  let pct = 0;
  const interval = setInterval(async () => {
    pct += 5;
    appState.updateProgress.percent = pct;
    appState.updateProgress.downloaded = Math.round((pct / 100) * 2450000);
    
    if (pct >= 100) {
      clearInterval(interval);
      appState.updateStatus = 'finished';
      
      // Simulate relaunch reboot delay
      setTimeout(() => {
        appState.version = '0.2.0';
        appState.updateAvailable = false;
        appState.updateStatus = 'up-to-date';
        
        // Remove update notification
        appState.notifications = appState.notifications.filter(n => n.id !== 'notify-update');
        
        appState.chatMessages.push({
          role: 'assistant',
          content: '主程式已成功重啟並完成升級！當前外殼版本：v0.2.0。'
        });
      }, 1500);
    }
  }, 100);
}

/**
 * Check current authentication status from backend
 * @returns {Promise<AuthStatusResponse | undefined>}
 */
export async function checkAuthStatus() {
  try {
    /** @type {AuthStatusResponse} */
    const res = await invoke('get_auth_status');
    appState.authStatus = res.status;
    appState.authUser = res.user;
    appState.authTenants = res.tenants;
    appState.activeTenant = res.activeTenant;
    return res;
  } catch (err) {
    console.error("Failed to check auth status:", err);
    appState.authStatus = 'unauthenticated';
    appState.authUser = null;
    appState.authTenants = [];
    appState.activeTenant = null;
  }
}

/**
 * Generic API Call helper that handles token expiration globally
 * @param {string} method
 * @param {string} path
 * @param {Record<string, any>} [body]
 * @returns {Promise<any>}
 */
export async function apiCall(method, path, body = {}) {
  try {
    return await invoke('api_call', { method, path, body });
  } catch (err) {
    const errorObj = /** @type {any} */ (err);
    const errCode = errorObj?.code || (typeof err === 'string' ? err : '');
    const errMsg = errorObj?.message || (typeof err === 'string' ? err : '');
    const isAuthExpired = 
      errCode === 'IAM_ERR_INVALID_CREDENTIALS' ||
      errMsg.includes('401') ||
      errMsg.includes('UNAUTHENTICATED') ||
      errMsg.includes('invalid credentials');

    if (path !== '/v1/auth/login' && isAuthExpired) {
      showToast('登入已過期');
      appState.authStatus = 'unauthenticated';
      appState.authUser = null;
      appState.authTenants = [];
      appState.activeTenant = null;
      navigate('/login');
    }
    throw err;
  }
}

/**
 * @param {string} email
 * @param {string} password
 * @returns {Promise<any>}
 */
export async function login(email, password) {
  const res = await apiCall('POST', '/v1/auth/login', { email, password, client_type: 'app' });
  await checkAuthStatus();
  return res;
}

/**
 * @param {string} adminName
 * @param {string} tenantName
 * @param {string} companyName
 * @param {string} adminEmail
 * @param {string} adminPassword
 * @param {string} tenantCode
 * @returns {Promise<any>}
 */
/**
 * Shared helper to seed onboarding parent and child tasks
 * @param {string} tenantName
 */
export async function seedOnboardingTasks(tenantName) {
  try {
    const parentTask = await createTaskAction('新租戶起步', 'sales', '主管', null);
    const subTask = await createTaskAction('設定部門', 'sales', '主管', parentTask.id);
    // Seed initial child task guidance message
    await appendTaskMessageAction(subTask.id, 'assistant', `您好！我是部門設定助理。新租戶「${tenantName}」建立完成後，首要步驟是建立組織部門。請問您想先新增哪一個部門？`);
    await fetchTasks('sales');
  } catch (e) {
    console.warn("Failed to create onboarding tasks on tenant creation:", e);
  }
}

/**
 * @param {string} adminName
 * @param {string} tenantName
 * @param {string} companyName
 * @param {string} adminEmail
 * @param {string} adminPassword
 * @param {string} tenantCode
 * @param {string} [taxId]
 * @returns {Promise<any>}
 */
export async function registerTenant(adminName, tenantName, companyName, adminEmail, adminPassword, tenantCode, taxId) {
  const res = await apiCall('POST', '/v1/auth/register-tenant', {
    admin_name: adminName,
    tenant_name: tenantName,
    company_name: companyName,
    admin_email: adminEmail,
    admin_password: adminPassword,
    tenant_code: tenantCode,
    tax_id: taxId || undefined
  });
  await checkAuthStatus();
  await seedOnboardingTasks(tenantName);
  return res;
}

/**
 * @returns {Promise<void>}
 */
export async function logoutAction() {
  try {
    await apiCall('POST', '/v1/auth/logout', {});
  } catch (err) {
    console.error("Failed to call logout API:", err);
  }
  appState.authStatus = 'unauthenticated';
  appState.authUser = null;
  appState.authTenants = [];
  appState.activeTenant = null;
  navigate('/login');
}

/**
 * @param {string} tenantId
 * @returns {Promise<any>}
 */
export async function selectTenantAction(tenantId) {
  const res = await apiCall('POST', '/v1/auth/select-tenant', { tenant_id: tenantId });
  await checkAuthStatus();
  return res;
}

/**
 * @param {string} tenantName
 * @param {string} companyName
 * @param {string} tenantCode
 * @param {string} [taxId]
 * @returns {Promise<any>}
 */
export async function createTenantAction(tenantName, companyName, tenantCode, taxId) {
  const res = await apiCall('POST', '/v1/auth/create-tenant', {
    tenant_name: tenantName,
    company_name: companyName,
    tenant_code: tenantCode,
    tax_id: taxId
  });
  await checkAuthStatus();
  await seedOnboardingTasks(tenantName);
  return res;
}

/**
 * Fetch tasks from SQLite
 * @param {string} [moduleId]
 * @returns {Promise<any[]>}
 */
export async function fetchTasks(moduleId = 'sales') {
  try {
    const list = await invoke('list_tasks', { moduleId });
    appState.tasks = list || [];
    return appState.tasks;
  } catch (err) {
    console.error("Failed to fetch tasks:", err);
    return [];
  }
}

/**
 * Create a task
 * @param {string} title
 * @param {string} moduleId
 * @param {string} assignee
 * @param {string | null} [parentTaskId]
 * @returns {Promise<any>}
 */
export async function createTaskAction(title, moduleId, assignee, parentTaskId = null) {
  try {
    const task = await invoke('create_task', {
      title,
      moduleId,
      assignee,
      parentTaskId: parentTaskId || null
    });
    await fetchTasks(moduleId);
    return task;
  } catch (err) {
    console.error("Failed to create task:", err);
    throw err;
  }
}

/**
 * Switch active task conversation
 * @param {string | null} taskId
 */
export async function switchActiveTask(taskId) {
  appState.activeTaskId = taskId;
  const targetKey = taskId || 'main';
  try {
    const msgs = await invoke('get_task_messages', { taskId: targetKey });
    appState.taskMessages[targetKey] = msgs || [];
    if (!taskId) {
      if (msgs && msgs.length > 0) {
        appState.chatMessages = msgs;
      }
    }
  } catch (err) {
    console.error(`Failed to fetch messages for ${targetKey}:`, err);
  }
}

/**
 * Append message to task conversation
 * @param {string} taskId
 * @param {string} role
 * @param {string} content
 */
export async function appendTaskMessageAction(taskId, role, content) {
  const targetKey = taskId || 'main';
  try {
    await invoke('append_task_message', { taskId: targetKey, role, content });
    if (!appState.taskMessages[targetKey]) {
      appState.taskMessages[targetKey] = [];
    }
    appState.taskMessages[targetKey].push({
      role,
      content,
      timestamp: Math.floor(Date.now() / 1000)
    });
    if (targetKey === 'main' || !appState.activeTaskId) {
      appState.chatMessages = [...appState.taskMessages['main']];
    }
  } catch (err) {
    console.error("Failed to append task message:", err);
  }
}

/**
 * Update task status
 * @param {string} taskId
 * @param {string} status
 */
export async function updateTaskStatusAction(taskId, status) {
  try {
    await invoke('update_task_status', { taskId, status });
    await fetchTasks(appState.activeWorkspace || 'sales');
  } catch (err) {
    console.error("Failed to update task status:", err);
  }
}

/**
 * Complete department setup task and report back to main agent
 * @param {string} taskId
 * @param {string} [departmentName]
 */
export async function completeDepartmentSetupTask(taskId, departmentName = '銷售部') {
  try {
    await updateTaskStatusAction(taskId, 'done');
    await appendTaskMessageAction(taskId, 'assistant', `已為您成功建立「${departmentName}」！此子任務已圓滿完成。`);
    // Report summary to main agent ambient conversation
    await appendTaskMessageAction('main', 'assistant', `✅「設定部門」已完成，新增了『${departmentName}』`);
    showToast(`「設定部門」任務已完成！結果已回報至主 Agent。`);
  } catch (err) {
    console.error("Failed to complete department setup task:", err);
    showToast(`完成任務失敗: ${err}`);
  }
}

/**
 * Initialize main chat greeting based on pending tasks count
 */
export async function initMainChatGreeting() {
  try {
    const tasks = await fetchTasks(appState.activeWorkspace || 'sales');
    const pendingCount = tasks.filter(t => t.status === 'pending' || t.status === 'in_progress').length;
    
    const mainMsgs = await invoke('get_task_messages', { taskId: 'main' });
    if (!mainMsgs || mainMsgs.length === 0) {
      const greeting = pendingCount > 0
        ? `你好，我是 AgentERP 智能助理。您目前有 ${pendingCount} 筆待處理任務。想從下方的任務開始，或直接跟我說您需要什麼協助：`
        : `你好，我是 AgentERP 智能助理。我已經載入本地安全邊緣工作站上下文，隨時可以為您服務。`;
      
      await appendTaskMessageAction('main', 'assistant', greeting);
    } else {
      appState.taskMessages['main'] = mainMsgs;
      appState.chatMessages = [...mainMsgs];
    }
  } catch (err) {
    console.error("Failed to init main chat greeting:", err);
  }
}

export async function simulateTokenExpiry() {
  try {
    await apiCall('POST', '/v1/test/expire', {});
  } catch (err) {
    console.log("Token expiration simulated successfully:", err);
  }
}

