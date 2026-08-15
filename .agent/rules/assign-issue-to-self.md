---
trigger: always_on
description: Rule to assign the active GitHub Issue to the agent during evaluation and planning
---

# GitHub Issue Auto-Assignment Rule

Whenever you are given a GitHub Issue to evaluate, plan, or implement:
1. 在確認 Issue 內容與規格已定案（非草稿或討論階段），且即將開始規劃或執行階段前，將該 Issue **指派給自己**。
2. Use the GitHub CLI `gh` to assign it:
   ```bash
   gh issue edit <issue-number> --add-assignee @me
   ```
3. Report that you have assigned the issue to yourself in your response.
