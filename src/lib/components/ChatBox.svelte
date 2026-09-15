<script>
  import { 
    appState, 
    switchActiveTask, 
    appendTaskMessageAction, 
    completeDepartmentSetupTask 
  } from '../store.svelte.js';
  import { Channel, invoke } from '../tauri.js';
  import { tick } from 'svelte';
  import QuickStartCards from './QuickStartCards.svelte';

  let inputVal = $state('');
  let chatEnd = $state(null);

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

  function handleBackToMain() {
    switchActiveTask(null);
  }

  function handleQuickPrompt(prompt) {
    inputVal = prompt;
    handleSend();
  }
</script>

<div class="chat-main-container">
  <!-- Chat Header Bar -->
  <div class="chat-header">
    <div class="header-title-group">
      {#if appState.activeTaskId}
        <button class="back-to-main-btn" onclick={handleBackToMain} title="返回主 Agent 環境對話">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="19" y1="12" x2="5" y2="12"></line>
            <polyline points="12 19 5 12 12 5"></polyline>
          </svg>
          <span>主環境</span>
        </button>
        <span class="pulse-icon subtask"></span>
        <h3 class="chat-title">{activeTask ? activeTask.title : '子任務對話'} (子 Agent)</h3>
      {:else}
        <span class="pulse-icon"></span>
        <h3 class="chat-title">AI Agent 協同對話</h3>
      {/if}
    </div>

    <div class="conversation-mode-badge">
      {#if appState.activeTaskId}
        <span>目前對話：子任務對話</span>
      {:else}
        <span>目前對話：一般業務詢問（尚未建立任務）</span>
      {/if}
    </div>
  </div>

  <!-- Messages Scroll Area -->
  <div class="chat-messages-viewport">
    {#each currentMessages as msg, idx}
      <div class="message-row {msg.role}">
        <div class="message-bubble {msg.role}">
          <div class="message-meta">
            {#if msg.role === 'user'}
              {appState.authUser?.display_name || appState.authUser?.email || '操作者'}
            {:else if appState.activeTaskId}
              {activeTask?.title} 子 Agent
            {:else}
              AgentERP 專家
            {/if}
          </div>
          <div class="message-body">{msg.content}</div>

          <!-- If first greeting in ambient conversation, render quick start cards under it -->
          {#if !appState.activeTaskId && idx === 0 && msg.role === 'assistant'}
            <QuickStartCards onSelectPrompt={handleQuickPrompt} />
          {/if}
        </div>
      </div>
    {/each}

    <!-- If no messages in ambient conversation, also render quick start cards -->
    {#if !appState.activeTaskId && currentMessages.length === 0}
      <div class="message-row assistant">
        <div class="message-bubble assistant">
          <div class="message-meta">AgentERP 專家</div>
          <div class="message-body">
            你好，我是 AgentERP 智能助理。已載入本地安全邊緣工作站上下文。您目前有 <strong>{appState.tasks.filter(t => t.status === 'pending' || t.status === 'in_progress').length}</strong> 筆待處理任務、<strong>{appState.notifications.length || 1}</strong> 則未讀通知。想從下面的常用任務開始，或直接跟我說您需要什麼協助：
          </div>
          <QuickStartCards onSelectPrompt={handleQuickPrompt} />
        </div>
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
          <div class="message-meta">{appState.activeTaskId ? (activeTask?.title + ' 子 Agent') : 'AgentERP 專家'} (串流中...)</div>
          <div class="message-body">{appState.currentStreamContent} <span class="cursor-blink">▋</span></div>
        </div>
      </div>
    {/if}
    <div bind:this={chatEnd}></div>
  </div>

  <!-- Chat Input Form -->
  <form class="chat-input-container" onsubmit={handleSend}>
    <div class="input-wrapper">
      <input 
        type="text" 
        class="chat-input-field" 
        placeholder={appState.activeTaskId ? `對「${activeTask?.title || '此任務'}」輸入指令...` : "輸入指令詢問 AI..."} 
        bind:value={inputVal}
        disabled={appState.isChatStreaming} 
      />
      <button 
        type="submit" 
        class="chat-send-btn" 
        aria-label="傳送指令" 
        disabled={appState.isChatStreaming || !inputVal.trim()}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
          <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
        </svg>
      </button>
    </div>
  </form>
</div>

<style>
  .chat-main-container {
    display: flex;
    flex-direction: column;
    flex-grow: 1;
    height: 100%;
    background: #0D0E12;
    overflow: hidden;
    position: relative;
  }

  .chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 24px;
    height: 48px;
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
    background: #0D0E12;
  }

  .header-title-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .chat-title {
    font-size: 0.96rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .back-to-main-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .back-to-main-btn:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--text-primary);
  }

  .pulse-icon {
    width: 8px;
    height: 8px;
    background: rgb(var(--accent-cyan));
    border-radius: 50%;
    box-shadow: 0 0 10px rgb(var(--accent-cyan));
    animation: pulse 2s infinite;
  }

  .pulse-icon.subtask {
    background: rgb(168, 85, 247);
    box-shadow: 0 0 10px rgb(168, 85, 247);
  }

  @keyframes pulse {
    0% { transform: scale(0.9); opacity: 0.6; }
    50% { transform: scale(1.3); opacity: 1; }
    100% { transform: scale(0.9); opacity: 0.6; }
  }

  .conversation-mode-badge {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-color);
    border-radius: 9999px;
    padding: 3px 12px;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .chat-messages-viewport {
    flex-grow: 1;
    overflow-y: auto;
    padding: 24px;
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
    max-width: 88%;
    padding: 16px 18px;
    border-radius: var(--radius-md);
    font-size: 0.95rem;
    line-height: 1.55;
  }

  .message-bubble.user {
    background: var(--accent);
    color: #0F0F12;
    border-bottom-right-radius: 2px;
  }

  .message-bubble.assistant {
    background: #181922;
    border: 1px solid var(--border-color);
    border-bottom-left-radius: 2px;
    width: 100%;
    max-width: 92%;
  }

  .message-meta {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--text-muted);
    margin-bottom: 6px;
  }

  .message-bubble.user .message-meta {
    color: rgba(0, 0, 0, 0.55);
  }

  .message-body {
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-primary);
  }

  .message-bubble.user .message-body {
    color: #0F0F12;
    font-weight: 500;
  }

  .subtask-quick-action {
    padding: 16px 20px;
    border-radius: var(--radius-md);
    border: 1px solid rgba(var(--accent-rgb), 0.35);
    background: rgba(var(--accent-rgb), 0.05);
    max-width: 92%;
  }

  .quick-action-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 6px;
  }

  .quick-action-desc {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 12px;
    line-height: 1.45;
  }

  .chat-input-container {
    padding: 16px 24px 20px;
    background: #0D0E12;
    border-top: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .input-wrapper {
    display: flex;
    align-items: center;
    background: #15161E;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 4px 6px 4px 16px;
    transition: all var(--transition-fast);
  }

  .input-wrapper:focus-within {
    border-color: rgba(var(--accent-rgb), 0.5);
    box-shadow: 0 0 0 2px rgba(var(--accent-rgb), 0.15);
  }

  .chat-input-field {
    flex-grow: 1;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.95rem;
    outline: none;
    padding: 8px 0;
  }

  .chat-input-field::placeholder {
    color: var(--text-muted);
  }

  .chat-send-btn {
    width: 36px;
    height: 36px;
    background: rgb(var(--accent-cyan));
    color: #0F0F12;
    border: none;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all var(--transition-fast);
    flex-shrink: 0;
  }

  .chat-send-btn:hover:not(:disabled) {
    background: rgba(var(--accent-cyan), 0.85);
    transform: scale(1.05);
  }

  .chat-send-btn:disabled {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-muted);
    cursor: not-allowed;
  }

  .cursor-blink {
    color: var(--accent);
    animation: blink 1s step-end infinite;
  }

  @keyframes blink {
    50% { opacity: 0; }
  }
</style>
