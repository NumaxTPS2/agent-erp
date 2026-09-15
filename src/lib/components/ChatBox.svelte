<script>
  import { 
    appState, 
    switchActiveTask, 
    appendTaskMessageAction, 
    completeDepartmentSetupTask 
  } from '../store.svelte.js';
  import { Channel, invoke } from '../tauri.js';
  import { tick } from 'svelte';

  let inputVal = $state('');
  let chatEnd = $state(null);
  let selectedModel = $state('gemini-3.5-flash');

  let activeTask = $derived(
    appState.activeTaskId ? appState.tasks.find(t => t.id === appState.activeTaskId) : null
  );

  let currentMessages = $derived(
    appState.activeTaskId 
      ? (appState.taskMessages[appState.activeTaskId] || [])
      : (appState.taskMessages['main'] && appState.taskMessages['main'].length > 0 
          ? appState.taskMessages['main'] 
          : appState.chatMessages)
  );

  let pendingTasks = $derived(
    appState.tasks.filter(t => t.status === 'pending' || t.status === 'in_progress')
  );

  // Auto scroll chat to bottom when message arrives
  $effect(() => {
    if (currentMessages.length || appState.currentStreamContent) {
      tick().then(() => {
        chatEnd?.scrollIntoView({ behavior: 'smooth' });
      });
    }
  });

  async function handleSend(e) {
    if (e) e.preventDefault();
    if (!inputVal.trim() || appState.isChatStreaming) return;

    const userMessage = inputVal.trim();
    inputVal = '';

    const currentTaskId = appState.activeTaskId;

    // Append user message
    await appendTaskMessageAction(currentTaskId, 'user', userMessage);

    // If inside "設定部門" subtask and user types to add department
    if (currentTaskId && activeTask && activeTask.title.includes('設定部門')) {
      if (activeTask.status !== 'done') {
        const deptName = userMessage.includes('部') ? userMessage.replace(/.*(新增|建立|要|加)/, '').trim() || '銷售部' : '銷售部';
        await completeDepartmentSetupTask(currentTaskId, deptName);
        return;
      }
    }

    // Default chat streaming response
    appState.isChatStreaming = true;
    appState.currentStreamContent = '';

    const channel = new Channel();
    channel.onmessage = async (chunk) => {
      appState.currentStreamContent += chunk.token;
      if (chunk.done) {
        const streamText = appState.currentStreamContent;
        appState.currentStreamContent = '';
        appState.isChatStreaming = false;
        await appendTaskMessageAction(currentTaskId, 'assistant', streamText);
      }
    };

    try {
      await invoke('simulate_agent_chat', {
        workspace: appState.activeWorkspace,
        message: userMessage,
        channel: channel
      });
    } catch (err) {
      console.error("Failed to stream chat:", err);
      await appendTaskMessageAction(currentTaskId, 'assistant', '對話串流連線失敗，請檢查 Rust 後端日誌。');
      appState.isChatStreaming = false;
    }
  }

  function handleTaskClick(taskId) {
    switchActiveTask(taskId);
  }

  function handleBackToMain() {
    switchActiveTask(null);
  }
</script>

<div class="chat-container">
  <div class="chat-header">
    {#if appState.activeTaskId}
      <div class="subtask-header-row">
        <button class="back-btn" onclick={handleBackToMain} title="返回主 Agent 環境對話">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="12" x2="5" y2="12"></line><polyline points="12 19 5 12 12 5"></polyline></svg>
          <span>主環境</span>
        </button>
        <div class="subtask-title-wrap">
          <span class="pulse-icon subtask"></span>
          <span class="subtask-title font-bold">{activeTask ? activeTask.title : '子任務對話'}</span>
        </div>
        {#if activeTask}
          <span class="badge {activeTask.status === 'done' ? 'badge-emerald' : 'badge-amber'}">
            {activeTask.status === 'done' ? '已完成' : '處理中'}
          </span>
        {/if}
      </div>
    {:else}
      <div class="chat-agent-title">
        <span class="pulse-icon"></span>
        <h3>AI Agent 協同對話（主環境）</h3>
      </div>
    {/if}

    <select class="model-select" bind:value={selectedModel}>
      <option value="gemini-3.5-flash">Gemini 3.5 Flash (BYOK)</option>
      <option value="deepseek-v3">DeepSeek V3 (BYOK)</option>
      <option value="openai-gpt4">OpenAI GPT-4o</option>
      <option value="ollama-local">Ollama Local (Ollama)</option>
    </select>
  </div>

  <div class="chat-messages-scroll">
    {#each currentMessages as msg}
      <div class="message-row {msg.role}">
        <div class="message-bubble {msg.role}">
          <div class="message-meta">
            {#if msg.role === 'user'}
              {appState.authUser?.display_name || appState.authUser?.email || '操作者'}
            {:else if appState.activeTaskId}
              {activeTask?.title} 子 Agent
            {:else}
              AgentERP 主助理
            {/if}
          </div>
          <div class="message-body">{msg.content}</div>
        </div>
      </div>
    {/each}

    <!-- Main Ambient Chat: Render Quick Task Cards if available and on main chat -->
    {#if !appState.activeTaskId && pendingTasks.length > 0}
      <div class="task-cards-container">
        <div class="task-cards-header">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path><rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect></svg>
          <span>待處理任務清單（點擊進入子 Agent 對話）：</span>
        </div>
        {#each appState.tasks as task}
          <button 
            type="button"
            class="task-card-item {task.status === 'done' ? 'task-done' : ''}" 
            onclick={() => handleTaskClick(task.id)}
          >
            <div class="task-card-top">
              <span class="task-card-title">{task.parent_task_id ? '└ ' + task.title : '📁 ' + task.title}</span>
              <span class="badge {task.status === 'done' ? 'badge-emerald' : 'badge-amber'}">
                {task.status === 'done' ? '已完成' : '待處理'}
              </span>
            </div>
            <div class="task-card-footer">
              <span>指派：{task.assignee}</span>
              <span class="task-enter-hint">進入對話 →</span>
            </div>
          </button>
        {/each}
      </div>
    {/if}

    <!-- Subtask View: Interactive Quick Action Button for Department Setup -->
    {#if appState.activeTaskId && activeTask && activeTask.title.includes('設定部門') && activeTask.status !== 'done'}
      <div class="subtask-quick-action glass-panel">
        <div class="quick-action-title">🏢 快速建立部門引導</div>
        <p class="quick-action-desc">點擊下方按鈕可快速建立「銷售部」並將結果自動摘要回報給主 Agent：</p>
        <button 
          class="btn btn-primary btn-sm" 
          onclick={() => completeDepartmentSetupTask(activeTask.id, '銷售部')}
        >
          ✨ 建立「銷售部」並完成任務
        </button>
      </div>
    {/if}

    {#if appState.isChatStreaming}
      <div class="message-row assistant streaming">
        <div class="message-bubble assistant">
          <div class="message-meta">{appState.activeTaskId ? (activeTask?.title + ' 子 Agent') : 'AgentERP 主助理'} (串流中...)</div>
          <div class="message-body">{appState.currentStreamContent} <span class="cursor-blink">▋</span></div>
        </div>
      </div>
    {/if}
    <div bind:this={chatEnd}></div>
  </div>

  <form class="chat-input-row" onsubmit={handleSend}>
    <input 
      type="text" 
      class="chat-input" 
      placeholder={appState.activeTaskId ? `對「${activeTask?.title || '此任務'}」輸入指令...` : "輸入指令詢問主 Agent..."} 
      bind:value={inputVal}
      disabled={appState.isChatStreaming} 
    />
    <button type="submit" class="chat-send-btn" aria-label="傳送訊息" disabled={appState.isChatStreaming || !inputVal.trim()}>
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="22" y1="2" x2="11" y2="13"></line><polygon points="22 2 15 22 11 13 2 9 22 2"></polygon></svg>
    </button>
  </form>
</div>

<style>
  .chat-container {
    width: 380px;
    border-left: 1px solid var(--border-color);
    background: var(--bg-secondary);
    display: flex;
    flex-direction: column;
    height: 100vh;
    flex-shrink: 0;
  }

  .chat-header {
    padding: 16px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .chat-agent-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .chat-agent-title h3 {
    font-size: 1rem;
    font-weight: 600;
  }

  .subtask-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .subtask-title-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-grow: 1;
    overflow: hidden;
  }

  .subtask-title {
    font-size: 0.95rem;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .back-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .back-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--text-primary);
  }

  .pulse-icon {
    width: 8px;
    height: 8px;
    background: rgb(var(--accent-cyan));
    border-radius: 50%;
    box-shadow: 0 0 8px rgb(var(--accent-cyan));
    animation: pulse 2s infinite;
  }

  .pulse-icon.subtask {
    background: rgb(147, 51, 234);
    box-shadow: 0 0 8px rgb(147, 51, 234);
  }

  @keyframes pulse {
    0% { transform: scale(0.9); opacity: 0.6; }
    50% { transform: scale(1.2); opacity: 1; }
    100% { transform: scale(0.9); opacity: 0.6; }
  }

  .model-select {
    width: 100%;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 0.85rem;
    font-family: inherit;
    outline: none;
  }

  .chat-messages-scroll {
    flex-grow: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .message-row {
    display: flex;
    width: 100%;
  }

  .message-row.user {
    justify-content: flex-end;
  }

  .message-bubble {
    max-width: 85%;
    padding: 12px 14px;
    border-radius: var(--radius-md);
    font-size: 0.95rem;
    line-height: 1.45;
  }

  .message-bubble.user {
    background: var(--accent);
    color: var(--bg-primary);
    border-bottom-right-radius: 2px;
  }

  .message-bubble.assistant {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-bottom-left-radius: 2px;
  }

  .message-meta {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .message-bubble.user .message-meta {
    color: rgba(0, 0, 0, 0.45);
  }

  .message-body {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .task-cards-container {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .task-cards-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .task-card-item {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .task-card-item:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(var(--accent-rgb), 0.4);
    transform: translateY(-1px);
  }

  .task-card-item.task-done {
    opacity: 0.7;
    background: rgba(16, 185, 129, 0.04);
    border-color: rgba(16, 185, 129, 0.2);
  }

  .task-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 4px;
  }

  .task-card-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .task-card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .task-enter-hint {
    color: var(--accent);
    font-weight: 500;
  }

  .subtask-quick-action {
    padding: 14px;
    border-radius: var(--radius-md);
    border: 1px solid rgba(var(--accent-rgb), 0.3);
    background: rgba(var(--accent-rgb), 0.05);
  }

  .quick-action-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .quick-action-desc {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin-bottom: 10px;
    line-height: 1.4;
  }

  .chat-input-row {
    padding: 16px;
    border-top: 1px solid var(--border-color);
    display: flex;
    gap: 8px;
    background: var(--bg-secondary);
  }

  .chat-input {
    flex-grow: 1;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    padding: 10px 14px;
    font-family: inherit;
    font-size: 0.95rem;
    outline: none;
  }

  .chat-input:focus {
    border-color: var(--accent);
  }

  .chat-send-btn {
    width: 40px;
    height: 40px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .chat-send-btn:hover:not(:disabled) {
    background: rgba(var(--accent-rgb), 0.8);
  }

  .chat-send-btn:disabled {
    background: var(--bg-tertiary);
    color: var(--text-muted);
    cursor: not-allowed;
    border: 1px solid var(--border-color);
  }

  .cursor-blink {
    color: var(--accent);
    animation: blink 1s step-end infinite;
  }

  @keyframes blink {
    50% { opacity: 0; }
  }
</style>

