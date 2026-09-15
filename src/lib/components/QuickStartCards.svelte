<script>
  /**
   * @type {{ onSelectPrompt?: (prompt: string) => void }}
   */
  let { onSelectPrompt } = $props();

  const cards = [
    {
      id: 'po_request',
      icon: '📝',
      title: '受理客戶採購需求',
      example: '例：中山國小要採購平板，可以接嗎？',
      prompt: '中山國小要採購平板，可以接嗎？',
      tag: '可示範',
      tagType: 'demo'
    },
    {
      id: 'credit_check',
      icon: '🔍',
      title: '查詢客戶信用/預算狀況',
      example: '例：大同國中現在還有多少預算？',
      prompt: '大同國中現在還有多少預算？',
      tag: '情境示意',
      tagType: 'mock'
    },
    {
      id: 'quote_compare',
      icon: '📊',
      title: '開立報價單並比對法人',
      example: '例：幫我比對四家法人的報價',
      prompt: '幫我比對四家法人的報價',
      tag: '情境示意',
      tagType: 'mock'
    },
    {
      id: 'stock_lookup',
      icon: '📦',
      title: '庫存查詢與調撥',
      example: '例：顯示螢幕庫存還夠嗎？',
      prompt: '顯示螢幕庫存還夠嗎？',
      tag: '情境示意',
      tagType: 'mock'
    },
    {
      id: 'billing_draft',
      icon: '🧾',
      title: '請款/對帳處理',
      example: '例：這筆請款草稿金額對嗎？',
      prompt: '這筆請款草稿金額對嗎？',
      tag: '情境示意',
      tagType: 'mock'
    },
    {
      id: 'help_faq',
      icon: '💬',
      title: '操作問題諮詢',
      example: '例：請款草稿核准要去找哪裡按？',
      prompt: '請款草稿核准要去找哪裡按？',
      tag: '情境示意',
      tagType: 'mock'
    }
  ];

  function handleCardClick(prompt) {
    if (typeof onSelectPrompt === 'function') {
      onSelectPrompt(prompt);
    }
  }
</script>

<div class="quick-start-grid">
  {#each cards as card}
    <button 
      type="button" 
      class="quick-card-btn" 
      onclick={() => handleCardClick(card.prompt)}
      title={`點擊執行「${card.title}」範例意圖`}
    >
      <div class="card-icon">{card.icon}</div>
      <div class="card-content">
        <div class="card-title-row">
          <span class="card-title">{card.title}</span>
        </div>
        <div class="card-example">{card.example}</div>
        <div class="card-tag-wrap">
          <span class="pill-tag {card.tagType}">{card.tag}</span>
        </div>
      </div>
    </button>
  {/each}
</div>

<style>
  .quick-start-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    width: 100%;
    margin-top: 12px;
  }

  .quick-card-btn {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 14px 16px;
    background: rgba(255, 255, 255, 0.025);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    transition: all var(--transition-fast);
    position: relative;
    overflow: hidden;
  }

  .quick-card-btn:hover {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(var(--accent-rgb), 0.35);
    transform: translateY(-1px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }

  .quick-card-btn:active {
    transform: translateY(0);
  }

  .card-icon {
    font-size: 1.3rem;
    line-height: 1;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .card-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex-grow: 1;
  }

  .card-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .card-title {
    font-size: 0.92rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .card-example {
    font-size: 0.78rem;
    color: var(--text-muted);
    line-height: 1.35;
  }

  .card-tag-wrap {
    margin-top: 4px;
  }

  .pill-tag {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 500;
    line-height: 1.2;
  }

  .pill-tag.demo {
    background: rgba(16, 185, 129, 0.15);
    color: rgb(52, 211, 153);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .pill-tag.mock {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-muted);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
</style>
