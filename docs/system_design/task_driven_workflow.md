# 任務驅動工作流程設計說明書 (Task-Driven Workflow)

本設計定義 AgentERP 的「任務（Task）」——一個橫跨所有模組的一等公民資料實體，是 AI Agent 協助使用者完成工作的核心單位，也是對話延續性（Continuity）的載體。

---

## 1. 核心概念：Task 是對話的容器，不只是待辦事項

傳統的「任務清單」只是狀態追蹤器（打勾、標記完成）。AgentERP 的 Task 更進一步：**每個 Task 綁定一段專屬的對話歷史**。使用者點擊某個 Task，看到的不是空白對話框，而是回到那個 Task 專屬的上下文——AI 記得之前討論到哪裡，使用者不需要重新描述背景。

這解決了純聊天介面的根本問題：對話一多就散掉、找不回脈絡。Task 把「這件事」跟「關於這件事的所有對話」綁在一起。

---

## 2. Task 資料模型

```typescript
interface Task {
  id: string;
  title: string;                  // 「大同國中智慧顯示設備採購」
  status: 'pending' | 'in_progress' | 'done' | 'cancelled';
  parentTaskId?: string | null;   // 父任務 id，頂層任務為 null
  moduleId: string;                // 所屬模組（對應 sidebar-rail 的任務類型）
  assignee: string;                 // 指派對象（顯示用字串，如「主管」「財會」「小華」）
  createdAt: number;
  completedAt?: number | null;
  conversationId: string;          // 綁定的對話串 id（見第 3 節）
}
```

- **父子任務**：`parentTaskId` 建立層級關係（例：「超預算核准」「請款草稿確認」是「大同國中智慧顯示設備採購」的子任務）。子任務可以指派給不同人、各自有獨立對話串。
- **狀態**：`pending`（待處理）、`in_progress`（進行中）、`done`（完成）、`cancelled`（使用者/業務判斷不需要做了）。父任務的完成不會自動由子任務狀態推導——由對應的業務邏輯決定是否要提示使用者確認完成。
- **失敗不是一種狀態**：單次嘗試失敗（例如邀請成員時 email 已存在）不會把 Task 標成某種「失敗」終態——任務本身還是要做，應留在 `pending`/`in_progress` 讓子 Agent 可以重試。失敗記錄在該 Task 的 `Conversation` 訊息裡（子 Agent 在對話中說明失敗原因），必要時另外透過 `notification_system.md` 發一個 `error` 類型通知，不會反映在 `status` 欄位上。

---

## 3. 對話與 Task 的綁定關係

對話不強制綁定 Task。使用者開啟某個模組時，預設進入一個**未綁定的環境對話（Ambient Conversation）**，畫面標示「目前對話：一般業務詢問（尚未建立任務）」。

```typescript
interface Conversation {
  id: string;
  taskId?: string | null;   // null = 尚未綁定任務的環境對話
  moduleId: string;
  messages: { role: 'user' | 'assistant'; content: string; timestamp: number }[];
}
```

**建立時機**：當 AI 判斷使用者的意圖已具體化為可追蹤的工作項目時（例如使用者描述了一筆具體採購案，不是單純查詢），主動詢問「要不要幫你建立一個任務追蹤這件事？」。使用者同意後：

1. 建立新的 `Task`
2. 把當前 `Conversation` 的 `taskId` 回填（該對話從此刻起歸屬這個 Task；已發生的訊息一併歸入，不會遺失前情）
3. 後續同一個 Task 的對話，都是同一條 `Conversation`，不會散成多條

系統也可以直接建立 Task 而不經過對話判斷（例如 M2 引導流程在租戶建立成功時自動建立「新租戶起步」任務，見第 6 節）——此時 Task 一開始沒有對話紀錄，AI 主動開口的第一句話就是這個 Task 底下的第一則訊息。

---

## 3.1. 主 Agent 與子 Agent：任務由子 Agent 代理，結果回報主 Agent

模組層級的環境對話（第 3 節的 Ambient Conversation）由**主 Agent** 負責——它是使用者切換模組時看到的那個持續身份（`agent_first_ux.md` 第 3 節的 systemPrompt/skills）。

每個 Task 一旦建立，實際協助使用者完成它的不是主 Agent 本人，而是一個**專屬這個 Task 的子 Agent**：

- 子 Agent 在該 Task 綁定的 `Conversation` 裡跟使用者互動，範疇收斂在這個 Task 本身——`systemPrompt` 可以比模組層級更具體（例：「你正在協助處理『超預算核准』這筆採購案的核准/拒絕，不需要處理其他業務」），`skills` 也可以只給這個任務需要的子集，不用整個模組的技能都開放
- 主 Agent **不需要持有子 Agent 的完整對話記錄**——上下文不會隨著任務數量增加而被拖大，每個子 Agent 各自輕量、聚焦
- 子 Agent 的任務結束時（狀態轉為 `done`/`cancelled`），把結果**摘要**回報給主 Agent 的環境對話（以系統訊息形式插入，例如：「✅「超預算核准」已由主管核准，金額 $XX」）——使用者即使沒有點進子任務，回到環境對話也能看到事情有進展，不需要自己去每個任務裡確認

這個關係跟第 4 節的任務面板互動一致：點擊任務卡片＝切換到該 Task 專屬的子 Agent 與其 `Conversation`；回到環境對話＝切回主 Agent，主 Agent 手上只有各子 Agent回報的摘要，不是原始逐字稿。

---

## 4. 任務面板（Task Panel）

常駐於側邊欄旁的獨立面板（可收合），顯示與目前使用者相關的任務：

- 標題列顯示未完成任務數量徽章（`pending` + `in_progress` 的數量總和，不含 `done`/`cancelled`）
- 每個任務卡片顯示：標題、狀態標籤、（若為子任務）父任務名稱、指派對象、（若已完成）完成時間
- 點擊任務卡片：載入該任務綁定的 `Conversation` 到 `agent-main`，若任務所屬模組跟目前不同，Shell 同步切換模組（systemPrompt/skills 一併替換，見 `agent_first_ux.md` 第 3 節）

---

## 5. AI 主動開場與快速起點卡片

使用者進入模組、尚未選定任何任務時，AI 的開場白主動報告現況，不是固定招呼詞：

> 你好，我是 AgentERP 智能助理。您目前有 **{n}** 筆待處理任務、**{m}** 則未讀通知。想從下面的常用任務開始，或直接跟我說您需要什麼協助：

下方渲染一組「常用任務類型」卡片（依模組宣告的技能範疇整理，非特定任務實例），使用者點擊即可帶著範例意圖起手；也可以完全忽略卡片、直接輸入自然語言。

---

## 6. 與通知系統的關係

`notification_system.md` 定義的通知是一次性的 FYI 推送；Task 是可追蹤、可恢復的工作單位。兩者透過通知的 `action` 欄位串接：一則通知可以將 `action.target` 指向一個 Task id，使用者點擊通知即直接跳轉並載入該 Task 的對話——但通知本身「已讀/關閉」的生命週期跟 Task 的 `status` 是獨立的兩件事，不要混用同一套狀態機。

「現況總覽」區塊分開列出「待處理任務數」與「未讀通知數」兩個計數，維持兩者概念獨立。

---

## 7. 套用範例：#73 新租戶起步引導

1. `create-tenant`/`register-tenant` 成功、且判定為全新租戶（見 #73）時，系統自動建立父任務「新租戶起步」（`status: in_progress`），以及三個子任務：「設定部門」「邀請成員」「指派角色」（皆為 `pending`）
2. 使用者進入主畫面，AI 開場白依第 5 節的格式，報告「3 筆待處理任務」，並在 `context-panel` 渲染一張 `action-list` 卡片（見 `agent_first_ux.md` 第 6 節）列出三個子任務
3. 使用者點擊某個子任務（例如「設定部門」），切換到該子任務的**子 Agent**，在其專屬 `Conversation` 裡用自然語言完成（例如「幫我新增一個部門」），子 Agent 執行後，該子任務狀態轉為 `done`
4. 子 Agent 把結果摘要回報給主 Agent 的環境對話（「✅「設定部門」已完成，新增了『銷售部』」），使用者不用切回子任務也能在環境對話看到進度
5. 三個子任務都 `done` 後，主 Agent 提示「新租戶起步」父任務是否可標記完成

---

## 8. 待確認事項

- Task 的持久化位置（本機 SQLite 或依 M1 的 mock/real 切換機制對接 TPS2）尚未定案，需要跟後端討論這是純前端本機概念、還是需要跨裝置同步的伺服器端資料
- AI 判斷「對話該不該提升為 Task」的具體規則（關鍵字？意圖分類？）需要在實作時進一步定義，本文件只定義機制與資料模型，不涵蓋判斷演算法細節
