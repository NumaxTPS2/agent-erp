// Detect if we are in Tauri runtime
export const isTauri = typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;

// Local mock states for browser context
let mockOrders = [
  {
    so_id: "SO-9921",
    customer_name: "A 公司 (Customer A)",
    po_reference: "PO-2026-0091",
    items: [{ name: "智能核心晶片 (AI Core Chip)", qty: 500, price: 120 }],
    total_amount: 60000.0,
    profit_margin: 0.25,
    capacity_usage: 0.85,
    status: "pending",
    created_at: Math.floor(Date.now() / 1000) - 3600
  }
];

let mockAuditLogs = [];
let mockInstalledModules = [];

// Local mock database for user authentication
let mockUsers = [
  { id: "usr_mock_admin", email: "admin@example.com", password: "password123", name: "Admin User" },
  { id: "usr_mock_new", email: "new@example.com", password: "password123", name: "New User" }
];
let mockTenants = [
  { id: "tnt_mock_1", code: "numax", name: "Numax Office", company_name: "Numax Inc." },
  { id: "tnt_mock_2", code: "alpha", name: "Alpha Corporation", company_name: "Alpha Corp." }
];
let mockUserTenants = [
  { user_id: "usr_mock_admin", tenant_id: "tnt_mock_1", role: "admin" },
  { user_id: "usr_mock_admin", tenant_id: "tnt_mock_2", role: "member" }
];
let mockSessions = null; // { token, user_id, active_tenant_id }

let mockLlmProviders = [
  { id: "openai", label: "OpenAI GPT-4o", base_url: "https://api.openai.com/v1", model_name: "gpt-4o", requires_key: true, has_key: false, active: true },
  { id: "deepseek", label: "DeepSeek V3 (BYOK)", base_url: "https://api.deepseek.com/v1", model_name: "deepseek-chat", requires_key: true, has_key: false, active: false },
  { id: "gemini", label: "Gemini (BYOK, OpenAI 相容端點)", base_url: "https://generativelanguage.googleapis.com/v1beta/openai", model_name: "gemini-2.0-flash", requires_key: true, has_key: false, active: false },
  { id: "ollama", label: "Ollama Local", base_url: "http://localhost:11434/v1", model_name: "llama3.1", requires_key: false, has_key: false, active: false }
];

const listeners = {};

// Mock Channel implementation for streaming chat in browser
export class Channel {
  constructor() {
    this.onmessage = null;
  }
}

// 1. Invoke wrapper
export async function invoke(cmd, args = {}) {
  if (isTauri) {
    const { invoke: realInvoke } = await import('@tauri-apps/api/core');
    return await realInvoke(cmd, args);
  }

  // Browser Mock Implementation
  console.log(`[Mock IPC] invoke: ${cmd}`, args);
  await new Promise(resolve => setTimeout(resolve, 300)); // Simulating latency

  switch (cmd) {
    case 'get_mirrored_orders':
      return [...mockOrders];
      
    case 'get_audit_logs':
      return [...mockAuditLogs];
      
    case 'get_installed_modules':
      return [...mockInstalledModules];
      
    case 'install_module': {
      const { moduleId, name, version, iconSvg, downloadUrl, sha256 } = args;
      if (!mockInstalledModules.some(m => m.id === moduleId)) {
        mockInstalledModules.push({
          id: moduleId,
          name,
          version,
          file_path: moduleId === 'crm' ? `/mock_cdn/${moduleId}_module.html` : `/mock_cdn/${moduleId}_module.js`,
          sha256,
          workspace: moduleId === 'sales_bi' ? 'finance' : moduleId,
          icon_svg: iconSvg
        });
      }
      return null;
    }
    
    case 'uninstall_module': {
      const { moduleId } = args;
      mockInstalledModules = mockInstalledModules.filter(m => m.id !== moduleId);
      return null;
    }
    
    case 'list_llm_providers':
      return [...mockLlmProviders];
      
    case 'set_active_llm_provider': {
      const { id } = args;
      mockLlmProviders = mockLlmProviders.map(p => ({ ...p, active: p.id === id }));
      return null;
    }
    
    case 'set_llm_api_key': {
      const { id, key } = args;
      mockLlmProviders = mockLlmProviders.map(p => p.id === id ? { ...p, has_key: !!key } : p);
      return null;
    }
    
    case 'simulate_webhook_order': {
      const exists = mockOrders.some(o => o.so_id === 'SO-9922');
      if (!exists) {
        const newOrder = {
          so_id: "SO-9922",
          customer_name: "A 公司 (Customer A)",
          po_reference: "PO-2026-0092",
          items: [{ name: "智能核心晶片 (AI Core Chip)", qty: 500, price: 120 }],
          total_amount: 60000.0,
          profit_margin: 0.25,
          capacity_usage: 0.85,
          status: "pending",
          created_at: Math.floor(Date.now() / 1000)
        };
        mockOrders.unshift(newOrder);
      }
      
      // Simulate event emission after 1.5 seconds
      setTimeout(() => {
        const payload = {
          id: "SO-9922",
          title: "A 公司採購單已送入",
          message: "系統已建立孿生訂單 SO-9922，等待安全確認。",
          workspace: "sales"
        };
        
        if (listeners['notification-hub']) {
          listeners['notification-hub'].forEach(cb => cb({ payload }));
        }
      }, 1500);
      
      return null;
    }
    
    case 'confirm_mutation': {
      const { mutationId, approved, operator } = args;
      const status = approved ? "approved" : "rejected";
      
      mockOrders = mockOrders.map(o => o.so_id === mutationId ? { ...o, status } : o);
      
      const timestamp = Math.floor(Date.now() / 1000);
      mockAuditLogs.unshift({
        id: `LOG-${timestamp}`,
        action_type: "confirm_order",
        arguments: { so_id: mutationId },
        decision: status,
        operator: operator || "Unknown",
        timestamp
      });
      return null;
    }
    
    case 'download_module': {
      const { moduleId } = args;
      if (moduleId === 'sales_bi') {
        return {
          id: 'sales_bi',
          name: 'Finance BI 大看板',
          version: '1.0.2',
          file_path: '/mock_cdn/sales_bi_module.js',
          iconSvg: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"></line><line x1="12" y1="20" x2="12" y2="4"></line><line x1="6" y1="20" x2="6" y2="14"></line></svg>',
          downloadUrl: 'sales_bi_module.js',
          sha256: 'mock-sha-sales-bi'
        };
      } else if (moduleId === 'crm') {
        return {
          id: 'crm',
          name: 'CRM 客戶模組',
          version: '1.0.1',
          file_path: '/mock_cdn/crm_module.html',
          iconSvg: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg>',
          downloadUrl: 'crm_dashboard.html',
          sha256: 'mock-sha-crm'
        };
      }
      return null;
    }
    
    case 'get_module_source': {
      const { moduleId } = args;
      const response = await fetch(`/mock_cdn/${moduleId}_module.js`);
      return await response.text();
    }
    
    case 'agent_chat': {
      const { workspace, message, history, channel } = args;
      if (channel && typeof channel.onmessage === 'function') {
        const fullResponse = `[瀏覽器模擬助理] 已收到您在 ${workspace} 工作區發送的訊息：「${message}」。由於當前運行於瀏覽器沙盒，此對話僅為流程展示。若需完整功能（包含真實 AI 模型呼叫、數據庫工具執行），請切換至 Tauri 端並設定 API 金鑰。`;
        
        let i = 0;
        const interval = setInterval(() => {
          if (i < fullResponse.length) {
            channel.onmessage({ token: fullResponse.slice(i, i + 3), done: false });
            i += 3;
          } else {
            clearInterval(interval);
            channel.onmessage({ token: '', done: true });
          }
        }, 30);
      }
      return null;
    }
    
    case 'get_auth_status': {
      if (!mockSessions) {
        return {
          status: "unauthenticated",
          user: null,
          tenants: [],
          activeTenant: null
        };
      }
      const user = mockUsers.find(u => u.id === mockSessions.user_id);
      const userTnts = mockUserTenants.filter(ut => ut.user_id === mockSessions.user_id);
      const tenants = userTnts.map(ut => {
        const t = mockTenants.find(tnt => tnt.id === ut.tenant_id);
        return {
          id: t.id,
          code: t.code,
          name: t.name,
          role: ut.role
        };
      });
      const activeTenant = tenants.find(t => t.id === mockSessions.active_tenant_id) || null;
      let status = "unauthenticated";
      if (tenants.length === 0) {
        status = "needs_tenant_creation";
      } else if (!activeTenant) {
        status = "needs_tenant_selection";
      } else {
        status = "authenticated";
      }
      return {
        status,
        user: user ? { id: user.id, email: user.email, display_name: user.name || user.email } : null,
        tenants,
        activeTenant
      };
    }
    
    case 'api_call': {
      const { method, path, body } = args;
      if (method === 'POST' && path === '/v1/auth/login') {
        const { email, password } = body;
        const user = mockUsers.find(u => u.email === email);
        if (!user || user.password !== password) {
          throw { code: "IAM_ERR_INVALID_CREDENTIALS", message: "invalid credentials" };
        }
        const userTnts = mockUserTenants.filter(ut => ut.user_id === user.id);
        const tenants = userTnts.map(ut => {
          const t = mockTenants.find(tnt => tnt.id === ut.tenant_id);
          return {
            id: t.id,
            code: t.code,
            name: t.name,
            role: ut.role
          };
        });
        mockSessions = {
          token: `mock-token-${user.id}`,
          user_id: user.id,
          active_tenant_id: tenants.length === 1 ? tenants[0].id : null
        };
        return {
          user: { id: user.id, email: user.email, display_name: user.name || user.email },
          tenants
        };
      }
      if (method === 'POST' && path === '/v1/auth/register-tenant') {
        const { admin_name, tenant_name, company_name, admin_email, admin_password, tenant_code } = body;
        if (!admin_name || !admin_name.trim()) {
          throw { code: "IAM_ERR_INVALID_ARGUMENT", message: "invalid argument: admin_name cannot be empty" };
        }
        if (admin_password.length < 8) {
          throw { code: "IAM_ERR_WEAK_PASSWORD", message: "weak password" };
        }
        if (mockUsers.some(u => u.email === admin_email)) {
          throw { code: "IAM_ERR_EMAIL_TAKEN", message: "email already taken" };
        }
        const user_id = `usr_${Date.now()}`;
        const tenant_id = `tnt_${Date.now()}`;
        mockUsers.push({ id: user_id, email: admin_email, password: admin_password, name: admin_name.trim() });
        mockTenants.push({ id: tenant_id, code: tenant_code, name: tenant_name, company_name });
        mockUserTenants.push({ user_id, tenant_id, role: "admin" });
        mockSessions = {
          token: `mock-token-${user_id}`,
          user_id,
          active_tenant_id: tenant_id
        };
        return {
          user: { id: user_id, email: admin_email, display_name: admin_name.trim() },
          tenants: [{ id: tenant_id, code: tenant_code, name: tenant_name, role: "admin" }]
        };
      }
      if (method === 'POST' && path === '/v1/auth/select-tenant') {
        const { tenant_id } = body;
        if (!mockSessions) {
          throw { code: "IAM_ERR_INVALID_CREDENTIALS", message: "auth: Session not found" };
        }
        const userTnts = mockUserTenants.filter(ut => ut.user_id === mockSessions.user_id);
        const isMember = userTnts.some(ut => ut.tenant_id === tenant_id);
        if (!isMember) {
          throw { code: "IAM_ERR_TENANT_NOT_ASSIGNED", message: "tenant not assigned" };
        }
        mockSessions.active_tenant_id = tenant_id;

        // Generate new mock scoped token
        const new_token = `mock-scoped-token-${mockSessions.user_id}`;
        mockSessions.token = new_token;

        return {
          access_token: new_token,
          refresh_token: `mock-refresh-${mockSessions.user_id}`,
          status: "authenticated"
        };
      }
      if (method === 'POST' && path === '/v1/auth/create-tenant') {
        const { tenant_name, company_name, tenant_code, tax_id } = body;
        if (!mockSessions) {
          throw { code: "IAM_ERR_INVALID_CREDENTIALS", message: "auth: Session not found" };
        }
        if (mockTenants.some(t => t.code === tenant_code)) {
          throw { code: "IAM_ERR_TENANT_CODE_TAKEN", message: "tenant code taken" };
        }
        const tenant_id = `tnt_${Date.now()}`;
        mockTenants.push({ id: tenant_id, code: tenant_code, name: tenant_name, company_name, tax_id });
        mockUserTenants.push({ user_id: mockSessions.user_id, tenant_id, role: "admin" });
        mockSessions.active_tenant_id = tenant_id;

        // Generate new mock scoped token
        const new_token = `mock-scoped-token-${mockSessions.user_id}`;
        mockSessions.token = new_token;

        const user = mockUsers.find(u => u.id === mockSessions.user_id);
        return {
          access_token: new_token,
          refresh_token: `mock-refresh-${mockSessions.user_id}`,
          user_id: mockSessions.user_id,
          email: user?.email || '',
          tenant_id,
          company_id: `cmp_${Date.now()}`
        };
      }
      if (method === 'POST' && path === '/v1/auth/logout') {
        mockSessions = null;
        return {};
      }
      if (method === 'POST' && path === '/v1/test/expire') {
        throw { code: "IAM_ERR_INVALID_CREDENTIALS", message: "invalid credentials" };
      }
      throw { code: "UNKNOWN_ERROR", message: `Mock API endpoint not found: ${method} ${path}` };
    }
    
    case 'logout': {
      mockSessions = null;
      return null;
    }
    
    default:
      console.warn(`Unhandled mock IPC command: ${cmd}`);
      return null;
  }
}

// 2. Listen wrapper
export async function listen(event, callback) {
  if (isTauri) {
    const { listen: realListen } = await import('@tauri-apps/api/event');
    return await realListen(event, callback);
  }

  if (!listeners[event]) {
    listeners[event] = [];
  }
  listeners[event].push(callback);
  
  return () => {
    listeners[event] = listeners[event].filter(cb => cb !== callback);
  };
}

// 3. Check updater wrapper
export async function check() {
  if (isTauri) {
    const { check: realCheck } = await import('@tauri-apps/plugin-updater');
    return await realCheck();
  }

  return {
    available: true,
    version: '0.2.0',
    body: '主要優化：\n1. 優化 Svelte 5 Runes 渲染引擎\n2. 升級 Rust 邊緣 SQLite 加密協議 (SQLCipher)\n3. 修正 Windows/macOS 系統更新偶發閃退問題。'
  };
}

// 4. Relaunch wrapper
export async function relaunch() {
  if (isTauri) {
    const { relaunch: realRelaunch } = await import('@tauri-apps/plugin-process');
    return await realRelaunch();
  }

  console.log('[Mock Process] Relaunching application...');
  alert('應用程式模擬重啟...');
  location.reload();
}
