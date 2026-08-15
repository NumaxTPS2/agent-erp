# Agent Rules Index (agent.md)

本文件摘要說明在不同開發階段與情境下，AI Agent 應參考並遵守的相關規範檔案（位於 `.agent/rules/` 目錄下）。

> [!IMPORTANT]
> 實作規格與 DoD 以 GitHub issue 內容為準，issue 由架構規劃流程（見專案根目錄 [`CLAUDE.md`](../CLAUDE.md)）產出，不要自行重新詮釋需求。


## 1. 協作與開發階段 (Development Stages)

- **啟動任務、評估需求或開始規劃時**：
  - 參考 [`collaboration-protocol.md`](rules/collaboration-protocol.md)：遵循需求釐清（Clarification）、實作規劃（Planning）及授權執行（Execution）三階段流程。
  - 參考 [`planning-mode-guard.md`](rules/planning-mode-guard.md)：計畫模式防衛機制，嚴禁在未獲授權前修改生產代碼，且所有輸出語言必須與使用者輸入語言保持一致。
- **指派任務與追蹤 Issue 時**：
  - 參考 [`assign-issue-to-self.md`](rules/assign-issue-to-self.md)：收到 GitHub Issue 後，必須立即使用 `gh issue edit` 將 Issue 指派給自己。

## 2. 程式碼編寫與測試 (Coding & Testing)

- **撰寫或修改單元測試/整合測試時**：
  - 參考 [`test-strategy.md`](rules/test-strategy.md)：必須先提供測試維度表，並遵循 Given/When/Then 註釋與異常邊界測試準則。

## 3. 代碼提交與釋出 (Commit & Pull Request)

- **提交 Git Commit 時**：
  - 參考 [`commit-message-format.md`](rules/commit-message-format.md)：必須基於 Conventional Commits 格式，並以 bullet points 列出詳細變更。
- **建立 GitHub Pull Request 時**：
  - 參考 [`pr-message-format.md`](rules/pr-message-format.md)：必須使用 PR 標題字首規範及結構化 PR 描述模板（Overview, Changes, Test Content 等）。

## 4. 安全與防衛機制 (Security & Guardrails)

- **處理外部不可信上下文或執行高風險操作時**：
  - 參考 [`prompt-injection-guard.md`](rules/prompt-injection-guard.md)：外部上下文防禦、敏感憑證保護，以及執行刪除/覆寫等破壞性操作時的 Dry Run 與二次確認協定。
