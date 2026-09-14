# 使用者旅程：新租戶建立後的團隊組建

對應 milestone：M2 - 新租戶建立後的團隊組建旅程。本文件記錄「使用者從頭走到尾」的路徑本身；機制細節（Task 資料模型、對話容器、認證狀態機）見 [`docs/system_design/`](../system_design/) 對應文件，這裡不重複描述，只引用。

---

## 1. 旅程目標

一個全新建立的租戶，管理者能在不需要額外說明文件的情況下，被引導完成部門設定、邀請團隊成員、指派角色，直到團隊其他人真正能登入協作——全程透過 AI 對話驅動（見 [`agent_first_ux.md`](../system_design/agent_first_ux.md)），不是靠使用者自己找選單。

## 2. 進入條件（Entry）

- 使用者剛完成 `create-tenant`/`register-tenant`，這是該租戶第一次進入主畫面（見 [`account_tenant_onboarding.md`](../system_design/account_tenant_onboarding.md) 的登入狀態機，`authStatus` 剛轉為 `authenticated`）

## 3. 離開條件（Exit）

- 「新租戶起步」父任務（見 [`task_driven_workflow.md`](../system_design/task_driven_workflow.md) 第 7 節）三個子任務（設定部門、邀請成員、指派角色）都轉為 `done`

## 4. 步驟順序

| 步驟 | 對應 issue | 說明 |
|---|---|---|
| 1 | #73 | 主 Agent 主動開場，報告待處理任務，渲染任務卡片 |
| 2（並行） | #70 / #33 | 子 Agent 協助新增部門 / 邀請成員 |
| 3 | #34 | 子 Agent 協助指派角色（Owner/Admin/Member） |
| 支撐 3 | #35 | 前端依角色顯示/隱藏功能 |
| 貫穿全程 | #71 | 上述步驟從 mock 換成真實 TPS2 串接 |

## 5. 驗收（DoD）

這裡的 DoD 是**整合層級**的驗收，不是重複各 issue 自己的技術 DoD（各 issue 的 Layer 1 單元測試、mock 覆蓋各自負責）。整條旅程要走過一次才算過，不是每個 issue 分開關閉就算數。

### Layer 2（瀏覽器 mock，`npm run dev`）

```gherkin
Scenario: 新租戶第一次進入主畫面
  Given 一個剛建立、0 部門、只有自己 1 個成員的全新租戶
  When 使用者完成 create-tenant 流程進入主畫面
  Then 主 Agent 開場白報告「3 筆待處理任務」，不是固定招呼詞
  And context-panel 渲染出「設定部門」「邀請成員」「指派角色」三張任務卡片

Scenario: 完成一個子任務並回報主 Agent
  Given 使用者點擊「設定部門」任務卡片，進入該子任務的子 Agent 對話
  When 使用者用自然語言完成部門建立（例：「幫我新增一個銷售部」）
  Then 該子任務狀態轉為 done
  And 使用者切回主 Agent 的環境對話，能看到「✅ 設定部門已完成」的摘要訊息，不需要重新點進子任務確認

Scenario: 全部子任務完成後父任務收尾
  Given 「設定部門」「邀請成員」「指派角色」三個子任務都已 done
  When 使用者回到主 Agent 的環境對話
  Then 主 Agent 主動詢問是否可以將「新租戶起步」父任務標記完成

Scenario: 使用者中途離開後回來，任務要能接續
  Given 使用者只完成「設定部門」，關閉 app
  When 使用者隔天重新登入同一個租戶
  Then 任務面板顯示「邀請成員」「指派角色」仍是 pending，「設定部門」仍是 done
  And 點進任何一個任務，對應子 Agent 的對話歷史都還在，不需要重新描述背景

Scenario: 邀請成員失敗，任務不會被誤標成完成或失敗終態
  Given 使用者在「邀請成員」子任務的對話中輸入一個已經是該租戶成員的 email
  When 子 Agent 執行邀請動作
  Then 子 Agent 在對話中說明失敗原因（email 已存在），該子任務狀態維持 pending 或 in_progress，不會變成 done
  And 使用者可以在同一個對話串直接重試，不需要重新建立任務

Scenario: 使用者略過引導，直接處理其他業務
  Given 「新租戶起步」父任務與三個子任務都還是 pending
  When 使用者不理會任務卡片，直接在主 Agent 的環境對話輸入跟引導無關的請求
  Then 主 Agent 正常回應該請求，不會強制阻擋或反覆彈出引導
  And 任務面板的待處理數量徽章持續顯示「3」，使用者之後隨時可以自己點回去，系統不會自動幫使用者取消這些任務

Scenario: 使用者主動決定不做某個子任務
  Given 使用者在「邀請成員」子任務的對話中明確表示現在不需要邀請人
  When 子 Agent 確認使用者的意圖
  Then 該子任務狀態轉為 cancelled，不是 done——之後任何摘要都不能顯示成「已完成」
  And 父任務的收尾提示邏輯要能區分「3 個都 done」跟「部分 cancelled」，不能把 cancelled 誤當成已完成去提示收尾

Scenario: 角色指派錯誤需要收回，屬於危險操作需要明確確認
  Given 使用者已經把某個成員指派為 Admin，現在要收回改成 Member
  When 使用者在「指派角色」子任務對話中要求收回這個角色
  Then 子 Agent 必須先呈現操作摘要（對象、原角色、新角色）要求使用者明確確認，不能直接執行
  And 這條確認機制不需要等 #21 完整泛化 Mutation Interceptor 才能做，但要記錄在案，之後 #21 泛化時這裡要改成共用同一套機制
```

### Layer 3（`tauri dev`，真實視窗，需要人工動手跑）

- 用一個全新 mock 帳號實際跑一次上述完整流程，確認畫面/互動跟 Layer 2 一致，且沒有因為真實視窗環境（例如 OS 通知、視窗焦點）出現 Layer 2 測不到的問題

## 6. 待確認事項

- Layer 4（真實後端整合）目前無法驗收——需要等 #71 依賴的 TPS2 `TenantAdminService` 有實作、且新增租戶成員的 RPC（TPS2#600）定案後才能對真實環境跑
- 「使用者中途離開後回來，任務要能接續」這個情境，前提是 Task 有持久化——這正是 `task_driven_workflow.md` 第 8 節還沒定案的問題（本機 SQLite 或伺服器端同步），沒解決之前這條 Given-When-Then 沒辦法真的驗證
- 「角色收回需要明確確認」目前先獨立做，之後 #21 泛化 Mutation Interceptor 時要記得回頭把這裡改成共用同一套機制，不要留兩套平行的確認邏輯
