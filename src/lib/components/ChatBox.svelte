<script>
  import { appState } from '../store.svelte.js';
  import { Channel, invoke } from '../tauri.js';
  import { tick } from 'svelte';

  let inputVal = $state('');
  let chatEnd = $state(null);
  let selectedModel = $state('gemini-3.5-flash');

  // Auto scroll chat to bottom when message arrives
  $effect(() => {
    if (appState.chatMessages.length || appState.currentStreamContent) {
      tick().then(() => {
        chatEnd?.scrollIntoView({ behavior: 'smooth' });
      });
    }
  });

  async function handleSend(e) {
    if (e) e.preventDefault();
    if (!inputVal.trim() || appState.isChatStreaming) return;

    const userMessage = inputVal;
    inputVal = '';

    // Add user message to history
    appState.chatMessages.push({ role: 'user', content: userMessage });
    
    // Set streaming state
    appState.isChatStreaming = true;
    appState.currentStreamContent = '';

    const channel = new Channel();
    channel.onmessage = (chunk) => {
      // Chunk is { token: string, done: boolean }
      appState.currentStreamContent += chunk.token;
      if (chunk.done) {
        appState.chatMessages.push({ role: 'assistant', content: appState.currentStreamContent });
        appState.currentStreamContent = '';
        appState.isChatStreaming = false;
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
      appState.chatMessages.push({ role: 'assistant', content: '對話串流連線失敗，請檢查 Rust 後端日誌。' });
      appState.isChatStreaming = false;
    }
  }
</script>

<div class="chat-container">
  <div class="chat-header">
    <div class="chat-agent-title">
      <span class="pulse-icon"></span>
      <h3>AI Agent 協同對話</h3>
    </div>
    <select class="model-select" bind:value={selectedModel}>
      <option value="gemini-3.5-flash">Gemini 3.5 Flash (BYOK)</option>
      <option value="deepseek-v3">DeepSeek V3 (BYOK)</option>
      <option value="openai-gpt4">OpenAI GPT-4o</option>
      <option value="ollama-local">Ollama Local (Ollama)</option>
    </select>
  </div>

  <div class="chat-messages-scroll">
    {#each appState.chatMessages as msg}
      <div class="message-row {msg.role}">
        <div class="message-bubble {msg.role}">
          <div class="message-meta">{msg.role === 'user' ? (appState.authUser?.display_name || appState.authUser?.email || '操作者') : 'AgentERP 專家'}</div>
          <div class="message-body">{msg.content}</div>
        </div>
      </div>
    {/each}

    {#if appState.isChatStreaming}
      <div class="message-row assistant streaming">
        <div class="message-bubble assistant">
          <div class="message-meta">AgentERP 專家 (串流中...)</div>
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
      placeholder="輸入指令詢問 AI..." 
      bind:value={inputVal}
      disabled={appState.isChatStreaming} 
    />
    <button type="submit" class="chat-send-btn" disabled={appState.isChatStreaming || !inputVal.trim()}>
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

  .pulse-icon {
    width: 8px;
    height: 8px;
    background: rgb(var(--accent-cyan));
    border-radius: 50%;
    box-shadow: 0 0 8px rgb(var(--accent-cyan));
    animation: pulse 2s infinite;
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
