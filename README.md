# Agent ERP

Tauri v2 + Svelte 5 桌面 ERP 應用，內建 AI 對話助理，整合外部後端 TPS2（Go + gRPC-Gateway）。

## 技術棧

- **前端**：Svelte 5 + Vite。`npm run dev` 走瀏覽器 mock 層即可本機開發，不需要先啟動 Tauri 視窗。
- **後端**：Rust（`src-tauri`），Tauri v2 command 作為 IPC 入口。
- **外部整合**：TPS2（Go + gRPC-Gateway），協定為 REST/JSON，對齊 grpc-gateway 曝露的 HTTP path。

## 架構

依賴方向單向：

```
Svelte UI 元件 → store.svelte.js → Tauri #[tauri::command] → Rust 業務邏輯 → 外部 gateway（reqwest / keyring / SQLite）
```

- **`api_call(method, path, body)`**：唯一的通用 API 轉發 command。依 `TPS2_BASE_URL` 環境變數是否設定，在本地 SQLite mock（`mock_dispatch`）與真實 TPS2（`call_real_tps2`）之間切換，前端不需要知道現在打的是 mock 還是真的後端。
- **Token 安全**：`access_token`/`refresh_token` 由 Rust 攔截後直接寫入 OS Keychain（`keyring` crate），永遠不會回傳給 JS。前端只能透過 `get_auth_status()` 拿到結構化狀態，問不到 token 本體。
- **導覽**：hash-based 路由（`appState.route` + `navigate(path)`），不使用 router 套件。

## 功能

各功能的詳細設計說明書放在 [`docs/system_design/`](./docs/system_design/)，README 只列索引，不重複內容：

- [`account_tenant_onboarding.md`](./docs/system_design/account_tenant_onboarding.md)：帳號登入、租戶選擇/建立的狀態機與真實後端對接方式
- 其餘既有設計文件（Shell 版面、模組安裝、通知系統等）見該目錄下其他檔案

## 開發

```bash
npm install
npm run dev        # 瀏覽器 mock 模式
npm run tauri dev  # 真實 Tauri 視窗
npm run check      # svelte-check 型別檢查
npm run build      # 前端靜態資源建置
```

對接真實 TPS2 環境時，設定 `TPS2_BASE_URL`（若環境有 Cloudflare Access 保護，另外設定 `CF_ACCESS_CLIENT_ID`/`CF_ACCESS_CLIENT_SECRET`）；不設定則自動使用本地 SQLite mock，不影響本機開發。

Rust 測試：

```bash
cd src-tauri && cargo test
```

## 規範文件

- [`CLAUDE.md`](./CLAUDE.md)：專案角色定位、架構原則、開發流程
- [`docs/standards/rust-backend.md`](./docs/standards/rust-backend.md)：Rust/Tauri 後端規範
- [`docs/standards/svelte-frontend.md`](./docs/standards/svelte-frontend.md)：Svelte 前端規範
- [`docs/standards/testing-verification.md`](./docs/standards/testing-verification.md)：四層驗證框架、已知錯誤碼邊界、驗收查核原則

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
