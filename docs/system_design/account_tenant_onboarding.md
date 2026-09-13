# 帳號與租戶起步設計說明書 (Account & Tenant Onboarding)

本設計定義使用者從登入到進入主畫面之間，需要決定「登入」與「租戶」兩件事的狀態機，以及這個狀態如何跟 TPS2 真實後端對接。

---

## 1. 登入流程狀態機

前端用 `appState.authStatus` 表示使用者當下卡在流程的哪個階段，四種狀態互斥：

| 狀態 | 意義 | 對應畫面 |
|---|---|---|
| `unauthenticated` | 未登入 | `LoginScreen` |
| `needs_tenant_creation` | 已登入，0 個租戶 | `OnboardingScreen`（建立公司） |
| `needs_tenant_selection` | 已登入，2 個以上租戶且尚未選定 | `TenantPicker` |
| `authenticated` | 已登入且租戶已確定 | 主畫面 |

狀態由 `get_auth_status()` 這支 Tauri command 計算，前端只消費計算結果，不自己推導。

---

## 2. 資料模型

**Tenant (1) : Company (N)**，Tenant 底下沒有獨立的 Group 這一層——Company 有 `tenant_id` 外鍵直接掛在 Tenant 底下。一個使用者可以同時屬於多個 Tenant（多租戶帳號），登入時如果剛好只有 1 個 Tenant，後端會自動把它塞進 token；0 個或多個 Tenant 時，token 不帶 tenant 範圍，要等使用者呼叫 `select-tenant`/`create-tenant` 後才會拿到範圍限定的新 token。

---

## 3. 與真實 TPS2 後端的對接

`execute_get_auth_status` 對接真實環境時呼叫 `GET /v1/auth/profile`，用回應裡的兩個欄位決定狀態，不用「剛好只有 1 個租戶」這種猜測式邏輯：

- **`loginStatus`**：後端直接算好的登入流程狀態，值域跟上面表格一致
- **`activeTenantId`**：目前 token 生效的租戶 id，用來在 `tenants` 陣列裡找出哪一筆是目前生效的，對應到 `AuthStatusResponse.active_tenant`

這兩個欄位跟 TPS2 另一個欄位 `status`（帳號本身是否被停用，例如 `active`）語意上完全獨立，不能混用。

`Login`/`RegisterTenant`/`GetProfile`/`SelectTenant`/`CreateTenant`/`Logout` 六支 API 對接時走同一套 token 攔截與錯誤碼轉換邏輯（見 [`rust-backend.md`](../standards/rust-backend.md)），沒有真實環境時自動退回本地 SQLite mock（`mock_dispatch`），前端完全感知不到差異。

---

## 4. 已知的流程限制

`create-tenant`（已登入使用者建立新租戶）目前允許每個帳號累積到一定數量的租戶（實際上限由後端決定），不是無限制；目前沒有「邀請既有使用者加入其他租戶」的 API，一個帳號要加入多個租戶只能透過自己重複呼叫 `create-tenant`，不是被別人邀請進去。
