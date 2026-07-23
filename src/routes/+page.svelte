<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import TreeNode from "$lib/components/TreeNode.svelte";
  import {
    createHistoryEntry,
    formatHistoryTime,
    type ClipboardHistoryEntry,
  } from "$lib/history";
  import {
    countNodes,
    findSearchMatches,
    formatBytes,
    formatJson,
  } from "$lib/json/format";
  import { analyzeJson } from "$lib/json/parser";
  import { repairJson } from "$lib/json/repair";
  import type { AnalysisResult } from "$lib/json/types";

  type ViewMode = "tree" | "pretty" | "raw";

  const emptyAnalysis: AnalysisResult = {
    inputText: "",
    documents: [],
    elapsedMs: 0,
  };

  let analysis = $state<AnalysisResult>(emptyAnalysis);
  let selectedId = $state("");
  let mode = $state<ViewMode>("tree");
  let searchQuery = $state("");
  let expansionLevel = $state(2);
  let expansionVersion = $state(0);
  let copiedMessage = $state("");
  let sourceLabel = $state("");
  let manualInput = $state("");
  let showManualInput = $state(false);
  let history = $state<ClipboardHistoryEntry[]>([]);
  let historyOpen = $state(false);
  let activeHistoryId = $state("");
  let repairBusy = $state(false);
  let repairError = $state("");
  let searchInput = $state<HTMLInputElement>();
  let rawErrorTextarea = $state<HTMLTextAreaElement>();
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;

  const currentDocument = $derived(
    analysis.documents.find((document) => document.id === selectedId) ??
      analysis.documents[0],
  );
  const prettyText = $derived(
    currentDocument ? formatJson(currentDocument.value) : "",
  );
  const matches = $derived(
    currentDocument
      ? findSearchMatches(currentDocument.value, searchQuery)
      : { direct: new Set<string>(), branches: new Set<string>(), count: 0 },
  );
  const nodeCount = $derived(
    currentDocument ? countNodes(currentDocument.value) : 0,
  );

  function isTauriRuntime(): boolean {
    return (
      typeof window !== "undefined" &&
      "__TAURI_INTERNALS__" in (window as unknown as Record<string, unknown>)
    );
  }

  function sourceDisplay(source: string): string {
    return (
      {
        shortcut: "快捷键",
        tray: "托盘",
        clipboard: "剪贴板",
        history: "历史",
        repair: "已修复",
        manual: "手动输入",
      }[source] ?? source
    );
  }

  async function loadClipboardHistory() {
    try {
      if (isTauriRuntime()) {
        history = await invoke<ClipboardHistoryEntry[]>("load_history");
      } else {
        history = JSON.parse(
          localStorage.getItem("showjson.clipboardHistory") ?? "[]",
        ) as ClipboardHistoryEntry[];
      }
    } catch (error) {
      copiedMessage = `读取历史失败：${String(error)}`;
    }
  }

  async function persistHistory(
    text: string,
    result: AnalysisResult,
    replaceId?: string,
  ) {
    const previous = replaceId
      ? history.find((entry) => entry.id === replaceId)
      : undefined;
    const entry = createHistoryEntry(text, result, previous);

    if (entry.bytes > 5 * 1024 * 1024) {
      copiedMessage = "内容超过 5 MB，未加入剪贴板历史";
      return;
    }

    try {
      if (isTauriRuntime()) {
        history = await invoke<ClipboardHistoryEntry[]>("save_history_entry", {
          entry,
        });
      } else {
        history = [
          entry,
          ...history.filter(
            (item) =>
              item.id !== entry.id && item.fingerprint !== entry.fingerprint,
          ),
        ].slice(0, 50);
        localStorage.setItem(
          "showjson.clipboardHistory",
          JSON.stringify(history),
        );
      }
      activeHistoryId = entry.id;
    } catch (error) {
      copiedMessage = String(error);
    }
  }

  function focusProblem() {
    const position = analysis.problem?.position;
    if (!rawErrorTextarea || position === undefined) return;
    rawErrorTextarea.focus();
    rawErrorTextarea.setSelectionRange(
      Math.max(0, position),
      Math.min(analysis.inputText.length, position + 1),
    );
  }

  function inspect(
    text: string,
    source = "clipboard",
    options: { recordHistory?: boolean; historyId?: string } = {},
  ) {
    const result = analyzeJson(text);
    analysis = result;
    selectedId = analysis.documents[0]?.id ?? "";
    mode = analysis.documents.length > 0 ? "tree" : "raw";
    searchQuery = "";
    sourceLabel = source;
    repairError = "";
    activeHistoryId = options.historyId ?? "";
    expansionVersion += 1;

    if (options.recordHistory !== false && source !== "manual" && source !== "history") {
      void persistHistory(text, result, options.historyId);
    }

    if (result.problem?.position !== undefined) {
      void tick().then(focusProblem);
    }
  }

  async function copyToClipboard(text: string, label: string) {
    try {
      if (isTauriRuntime()) {
        await invoke("copy_text", { text });
      } else {
        await navigator.clipboard.writeText(text);
      }
      copiedMessage = `${label}已复制`;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copiedMessage = ""), 1500);
    } catch (error) {
      copiedMessage = `复制失败：${String(error)}`;
    }
  }

  async function openHistoryEntry(entry: ClipboardHistoryEntry) {
    historyOpen = false;
    inspect(entry.text, "history", {
      recordHistory: false,
      historyId: entry.id,
    });
  }

  async function deleteHistoryEntry(event: MouseEvent, id: string) {
    event.stopPropagation();
    try {
      if (isTauriRuntime()) {
        history = await invoke<ClipboardHistoryEntry[]>("delete_history_entry", {
          id,
        });
      } else {
        history = history.filter((entry) => entry.id !== id);
        localStorage.setItem(
          "showjson.clipboardHistory",
          JSON.stringify(history),
        );
      }
      if (activeHistoryId === id) activeHistoryId = "";
    } catch (error) {
      copiedMessage = `删除失败：${String(error)}`;
    }
  }

  async function clearClipboardHistory() {
    if (!window.confirm("确认清空全部剪贴板历史？此操作无法撤销。")) return;
    try {
      if (isTauriRuntime()) {
        await invoke("clear_history");
      } else {
        localStorage.removeItem("showjson.clipboardHistory");
      }
      history = [];
      activeHistoryId = "";
    } catch (error) {
      copiedMessage = `清空失败：${String(error)}`;
    }
  }

  async function attemptRepair() {
    if (!analysis.problem || !analysis.inputText) return;
    repairBusy = true;
    repairError = "";

    try {
      const repairedText = repairJson(analysis.inputText);
      const repairedAnalysis = analyzeJson(repairedText);
      if (repairedAnalysis.documents.length === 0) {
        throw new Error(
          repairedAnalysis.problem?.message ?? "修复结果仍然不是有效 JSON",
        );
      }

      const historyId = activeHistoryId;
      inspect(repairedText, "repair", {
        recordHistory: false,
        historyId,
      });
      await copyToClipboard(repairedText, "修复后的 JSON");
      await persistHistory(repairedText, repairedAnalysis, historyId || undefined);
      sourceLabel = "repair";
    } catch (error) {
      repairError = String(error);
    } finally {
      repairBusy = false;
    }
  }

  function setExpansion(level: number) {
    expansionLevel = level;
    expansionVersion += 1;
  }

  async function hideWindow() {
    if (isTauriRuntime()) {
      await invoke("hide_window");
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      void hideWindow();
      return;
    }

    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "f") {
      event.preventDefault();
      searchInput?.focus();
      searchInput?.select();
      return;
    }

  }

  onMount(() => {
    const unlisteners: UnlistenFn[] = [];
    window.addEventListener("keydown", handleKeydown);

    if (isTauriRuntime()) {
      void Promise.all([
        listen<{ text: string; source: string }>(
          "showjson://clipboard",
          (event) => inspect(event.payload.text, event.payload.source),
        ),
        listen<string>("showjson://error", (event) => {
          analysis = {
            inputText: "",
            documents: [],
            problem: { message: event.payload },
            elapsedMs: 0,
          };
        }),
      ]).then((listeners) => unlisteners.push(...listeners));
    }
    void loadClipboardHistory();

    return () => {
      window.removeEventListener("keydown", handleKeydown);
      unlisteners.forEach((unlisten) => unlisten());
      if (copiedTimer) clearTimeout(copiedTimer);
    };
  });
</script>

<svelte:head>
  <meta
    name="description"
    content="ShowJSON — 从任意应用快速查看剪贴板中的 JSON"
  />
</svelte:head>

<main class="app-shell">
  <header class="app-header">
    <div class="brand">
      <span class="brand-mark" aria-hidden="true"><span>{`{`}</span><span>{`}`}</span></span>
      <div>
        <h1>ShowJSON</h1>
        <p>本地快速 JSON 查看器</p>
      </div>
    </div>

    <div class="header-actions">
      <button
        class:active={historyOpen}
        class="history-button"
        onclick={() => (historyOpen = !historyOpen)}
      >
        <span aria-hidden="true">◷</span>
        历史
        {#if history.length > 0}<b>{history.length}</b>{/if}
      </button>
      <div class="shortcut-hint" title="读取剪贴板的唯一入口">
        <span class="capture-icon" aria-hidden="true">⌘</span>
        查看剪贴板
        <kbd>⌘/Ctrl ⇧ J</kbd>
      </div>
    </div>
  </header>

  {#if historyOpen}
    <button
      class="history-backdrop"
      aria-label="关闭剪贴板历史"
      onclick={() => (historyOpen = false)}
    ></button>
    <aside class="history-panel">
      <div class="history-header">
        <div>
          <h2>剪贴板历史</h2>
          <p>仅记录主动读取的文本，保存在本机</p>
        </div>
        <button aria-label="关闭" onclick={() => (historyOpen = false)}>×</button>
      </div>

      {#if history.length > 0}
        <div class="history-list">
          {#each history as entry (entry.id)}
            <div
              class:active={activeHistoryId === entry.id}
              class="history-item"
              onclick={() => openHistoryEntry(entry)}
              onkeydown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  void openHistoryEntry(entry);
                }
              }}
              role="button"
              tabindex="0"
            >
              <span class:valid={entry.valid} class="history-status"></span>
              <span class="history-content">
                <strong>{entry.preview}</strong>
                <small>
                  {formatHistoryTime(entry.createdAt)}
                  <i>·</i>
                  {formatBytes(entry.text)}
                  <i>·</i>
                  {entry.valid ? entry.kind : "解析失败"}
                </small>
              </span>
              <button
                class="delete-history"
                aria-label="删除此条历史"
                title="删除"
                onclick={(event) => deleteHistoryEntry(event, entry.id)}
              >×</button>
            </div>
          {/each}
        </div>
        <div class="history-footer">
          <span>最多 50 条 / 25 MB</span>
          <button onclick={clearClipboardHistory}>清空全部</button>
        </div>
      {:else}
        <div class="history-empty">
          <span>◷</span>
          <p>还没有剪贴板历史</p>
          <small>按快捷键查看后会自动记录</small>
        </div>
      {/if}
    </aside>
  {/if}

  {#if currentDocument}
    {#if analysis.documents.length > 1}
      <nav class="document-tabs" aria-label="JSON 片段">
        {#each analysis.documents as document}
          <button
            class:active={selectedId === document.id}
            onclick={() => {
              selectedId = document.id;
              searchQuery = "";
              expansionVersion += 1;
            }}
          >
            {document.label}
          </button>
        {/each}
      </nav>
    {/if}

    <section class="toolbar">
      <div class="segmented" aria-label="查看模式">
        <button class:active={mode === "tree"} onclick={() => (mode = "tree")}>Tree</button>
        <button class:active={mode === "pretty"} onclick={() => (mode = "pretty")}>Pretty</button>
        <button class:active={mode === "raw"} onclick={() => (mode = "raw")}>Raw</button>
      </div>

      <label class="search-box">
        <span aria-hidden="true">⌕</span>
        <input
          bind:this={searchInput}
          bind:value={searchQuery}
          placeholder="搜索 Key 或 Value"
          aria-label="搜索 Key 或 Value"
        />
        {#if searchQuery}
          <span class="match-count">{matches.count}</span>
          <button
            aria-label="清空搜索"
            class="clear-search"
            onclick={() => (searchQuery = "")}
          >×</button>
        {:else}
          <kbd>⌘F</kbd>
        {/if}
      </label>

      <div class="toolbar-spacer"></div>

      {#if mode === "tree"}
        <div class="depth-control">
          <span>展开</span>
          {#each [1, 2, 3] as level}
            <button
              class:active={expansionLevel === level}
              onclick={() => setExpansion(level)}
            >{level}</button>
          {/each}
          <button
            class:active={expansionLevel === 999}
            onclick={() => setExpansion(999)}
          >All</button>
          <button onclick={() => setExpansion(0)}>收起</button>
        </div>
      {/if}

      <button
        class="toolbar-button"
        onclick={() => copyToClipboard(prettyText, "格式化 JSON")}
      >复制 JSON</button>
    </section>

    <section class="viewer">
      {#if mode === "tree"}
        <div class="tree-view">
          <TreeNode
            value={currentDocument.value}
            name="$"
            path={[]}
            depth={0}
            {expansionLevel}
            {expansionVersion}
            {matches}
            query={searchQuery}
            onCopy={copyToClipboard}
          />
        </div>
      {:else}
        <pre class="text-view"><code>{mode === "pretty" ? prettyText : currentDocument.rawText}</code></pre>
      {/if}
    </section>

    <footer class="status-bar">
      <div class="status-left">
        <span class="success-dot"></span>
        <span>{currentDocument.label}</span>
        <span class="separator">·</span>
        <span>{formatBytes(currentDocument.rawText)}</span>
        <span class="separator">·</span>
        <span>{nodeCount.toLocaleString()} nodes</span>
        <span class="separator">·</span>
        <span>{analysis.elapsedMs.toFixed(1)} ms</span>
        {#if sourceLabel}
          <span class="separator">·</span>
          <span>{sourceDisplay(sourceLabel)}</span>
        {/if}
      </div>
      <div class="privacy"><span>●</span> 数据仅在本机处理</div>
    </footer>
  {:else if analysis.problem && analysis.inputText}
    <section class="error-state">
      <div class="error-header">
        <div class="error-title">
          <span class="problem-icon">!</span>
          <div>
            <h2>无法解析这段文本</h2>
            <p>{analysis.problem.message}</p>
          </div>
        </div>
        <div class="error-actions">
          {#if analysis.problem.position !== undefined}
            <button class="secondary-action" onclick={focusProblem}>
              定位问题
              {#if analysis.problem.line}
                <span>第 {analysis.problem.line} 行，第 {analysis.problem.column} 列</span>
              {/if}
            </button>
          {/if}
          <button
            class="secondary-action"
            onclick={() => copyToClipboard(analysis.inputText, "原文")}
          >复制原文</button>
          <button
            class="repair-button"
            onclick={attemptRepair}
            disabled={repairBusy}
            title="修复成功后会覆盖当前内容、系统剪贴板和对应历史记录"
          >{repairBusy ? "正在修复…" : "尝试修复并覆盖"}</button>
        </div>
      </div>

      {#if repairError}
        <div class="repair-error">
          <strong>无法自动修复</strong>
          <span>{repairError}</span>
        </div>
      {/if}

      <div class="raw-problem-shell">
        <div class="raw-problem-toolbar">
          <span>原始文本</span>
          <span>{formatBytes(analysis.inputText)}</span>
        </div>
        <textarea
          bind:this={rawErrorTextarea}
          class="raw-problem-text"
          value={analysis.inputText}
          readonly
          spellcheck="false"
          aria-label="无法解析的原始文本"
        ></textarea>
      </div>

      <footer class="error-footer">
        <span>ShowJSON 不会自动修改输入；只有点击修复按钮后才会覆盖。</span>
        <span>复制新内容后按 <kbd>⌘/Ctrl ⇧ J</kbd> 重新读取</span>
      </footer>
    </section>
  {:else}
    <section class="empty-state">
      <div class="empty-visual" aria-hidden="true">
        <span class="brace left">{`{`}</span>
        <div class="json-lines">
          <i></i><i></i><i></i>
        </div>
        <span class="brace right">{`}`}</span>
      </div>

      {#if analysis.problem}
        <div class="problem-card">
          <span class="problem-icon">!</span>
          <div>
            <h2>暂时无法识别为 JSON</h2>
            <p>{analysis.problem.message}</p>
            {#if analysis.problem.line}
              <p>位置：第 {analysis.problem.line} 行，第 {analysis.problem.column} 列</p>
            {/if}
          </div>
        </div>
      {:else}
        <h2>复制 JSON，一键查看</h2>
        <p class="empty-description">
          在浏览器、终端或任意应用中复制文本，然后按全局快捷键。
        </p>
      {/if}

      <div class="primary-action">
        <span>复制文本后按快捷键查看</span>
        <kbd>⌘/Ctrl ⇧ J</kbd>
      </div>

      <button
        class="manual-toggle"
        onclick={() => (showManualInput = !showManualInput)}
      >{showManualInput ? "收起手动输入" : "也可以直接粘贴文本"}</button>

      {#if showManualInput}
        <div class="manual-input">
          <textarea
            bind:value={manualInput}
            placeholder="粘贴 JSON 或包含 JSON 的日志"
            spellcheck="false"
          ></textarea>
          <button onclick={() => inspect(manualInput, "manual")} disabled={!manualInput.trim()}>
            解析文本
          </button>
        </div>
      {/if}

      <div class="feature-pills">
        <span>标准 JSON</span>
        <span>转义 JSON</span>
        <span>日志片段</span>
        <span>JSON Lines</span>
        <span>大整数无损</span>
      </div>
    </section>
  {/if}

  {#if copiedMessage}
    <div class="toast" role="status">{copiedMessage}</div>
  {/if}
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(:root) {
    font-family:
      Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI",
      "PingFang SC", "Microsoft YaHei", sans-serif;
    color: #e7e9ed;
    background: #0f1216;
    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    --surface: #0f1216;
    --surface-raised: #181c22;
    --surface-hover: #20252c;
    --surface-subtle: #14181d;
    --border: #292f37;
    --border-strong: #414954;
    --text-primary: #e7e9ed;
    --text-secondary: #abb2bd;
    --text-muted: #727b87;
    --accent: #58d6a6;
    --accent-strong: #79e8bd;
    --accent-muted: rgba(88, 214, 166, 0.12);
    --search-bg: rgba(255, 197, 76, 0.14);
    --json-key: #7bb8ff;
    --json-string: #a8dc8e;
    --json-number: #e9ad73;
    --json-boolean: #cf9cff;
    --json-null: #7f8996;
  }

  :global(html),
  :global(body) {
    margin: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--surface);
  }

  :global(button),
  :global(input),
  :global(textarea) {
    font: inherit;
  }

  :global(button:focus-visible),
  :global(input:focus-visible),
  :global(textarea:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .app-shell {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background:
      radial-gradient(circle at 18% -10%, rgba(57, 111, 94, 0.12), transparent 32%),
      var(--surface);
  }

  .app-header {
    flex: 0 0 68px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 18px 10px 20px;
    border-bottom: 1px solid var(--border);
    background: rgba(15, 18, 22, 0.92);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .history-button {
    height: 38px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-secondary);
    background: var(--surface-raised);
    cursor: pointer;
    font-size: 12px;
  }

  .history-button:hover,
  .history-button.active {
    border-color: var(--border-strong);
    color: var(--text-primary);
    background: var(--surface-hover);
  }

  .history-button b {
    min-width: 18px;
    padding: 1px 5px;
    border-radius: 999px;
    color: var(--accent-strong);
    background: var(--accent-muted);
    font-size: 9px;
    font-weight: 600;
  }

  .history-backdrop {
    position: fixed;
    inset: 68px 0 0;
    z-index: 29;
    border: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    cursor: default;
  }

  .history-panel {
    position: fixed;
    top: 68px;
    right: 0;
    bottom: 0;
    z-index: 30;
    width: min(390px, 88vw);
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border-strong);
    background: #12161b;
    box-shadow: -18px 0 48px rgba(0, 0, 0, 0.36);
  }

  .history-header {
    flex: 0 0 70px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px 10px 16px;
    border-bottom: 1px solid var(--border);
  }

  .history-header h2,
  .history-header p {
    margin: 0;
  }

  .history-header h2 {
    color: var(--text-primary);
    font-size: 14px;
  }

  .history-header p {
    margin-top: 3px;
    color: var(--text-muted);
    font-size: 10px;
  }

  .history-header > button {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 6px;
    color: var(--text-muted);
    background: transparent;
    cursor: pointer;
    font-size: 18px;
  }

  .history-header > button:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
  }

  .history-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px;
  }

  .history-item {
    width: 100%;
    min-height: 62px;
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 10px 8px;
    border: 1px solid transparent;
    border-radius: 8px;
    text-align: left;
    cursor: pointer;
  }

  .history-item:hover,
  .history-item.active {
    border-color: var(--border);
    background: var(--surface-hover);
  }

  .history-item.active {
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .history-status {
    width: 7px;
    height: 7px;
    flex: 0 0 auto;
    margin-top: 5px;
    border-radius: 50%;
    background: #bd744d;
  }

  .history-status.valid {
    background: var(--accent);
  }

  .history-content {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .history-content strong {
    display: -webkit-box;
    overflow: hidden;
    color: var(--text-secondary);
    font: 11px/1.4 ui-monospace, SFMono-Regular, Menlo, monospace;
    overflow-wrap: anywhere;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .history-content small {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--text-muted);
    font-size: 9px;
  }

  .history-content i {
    color: #444c56;
    font-style: normal;
  }

  .delete-history {
    width: 24px;
    height: 24px;
    flex: 0 0 auto;
    border: 0;
    border-radius: 5px;
    color: var(--text-muted);
    background: transparent;
    cursor: pointer;
    opacity: 0;
  }

  .history-item:hover .delete-history,
  .delete-history:focus-visible {
    opacity: 1;
  }

  .delete-history:hover {
    color: #f1a57d;
    background: rgba(189, 116, 77, 0.12);
  }

  .history-footer {
    flex: 0 0 44px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 0 16px;
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 9px;
  }

  .history-footer button {
    border: 0;
    color: #c18a70;
    background: transparent;
    cursor: pointer;
    font-size: 10px;
  }

  .history-footer button:hover {
    color: #efa47e;
  }

  .history-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }

  .history-empty > span {
    margin-bottom: 10px;
    color: #4b5662;
    font-size: 38px;
  }

  .history-empty p {
    margin: 0 0 4px;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .history-empty small {
    font-size: 10px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 11px;
  }

  .brand-mark {
    width: 38px;
    height: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 1px;
    border: 1px solid rgba(88, 214, 166, 0.28);
    border-radius: 10px;
    color: var(--accent);
    background: var(--accent-muted);
    font: 700 18px/1 ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .brand h1,
  .brand p {
    margin: 0;
  }

  .brand h1 {
    color: var(--text-primary);
    font-size: 16px;
    line-height: 20px;
    letter-spacing: 0.01em;
  }

  .brand p {
    color: var(--text-muted);
    font-size: 11px;
    line-height: 15px;
  }

  .shortcut-hint,
  .primary-action {
    border: 1px solid rgba(88, 214, 166, 0.45);
    border-radius: 8px;
    color: #dff9ef;
    background: linear-gradient(180deg, #247a60, #1b624d);
    box-shadow: 0 4px 14px rgba(14, 80, 60, 0.22);
  }

  .shortcut-hint {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 9px 0 12px;
    font-size: 12px;
  }

  .capture-icon {
    color: var(--accent-strong);
    font-size: 13px;
  }

  kbd {
    border: 1px solid rgba(255, 255, 255, 0.13);
    border-radius: 5px;
    padding: 2px 5px;
    color: #bdc7d2;
    background: rgba(0, 0, 0, 0.22);
    font: 10px/16px ui-monospace, SFMono-Regular, Menlo, monospace;
    white-space: nowrap;
  }

  .document-tabs {
    flex: 0 0 36px;
    display: flex;
    align-items: end;
    gap: 2px;
    overflow-x: auto;
    padding: 4px 12px 0;
    border-bottom: 1px solid var(--border);
    background: var(--surface-subtle);
  }

  .document-tabs button {
    max-width: 260px;
    overflow: hidden;
    padding: 7px 12px 8px;
    border: 0;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
    background: transparent;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
    font-size: 11px;
  }

  .document-tabs button.active {
    border-bottom-color: var(--accent);
    color: var(--text-primary);
  }

  .toolbar {
    flex: 0 0 50px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-subtle);
  }

  .segmented {
    display: inline-flex;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: #0e1115;
  }

  .segmented button,
  .depth-control button,
  .toolbar-button {
    border: 0;
    border-radius: 5px;
    color: var(--text-secondary);
    background: transparent;
    cursor: pointer;
    font-size: 11px;
  }

  .segmented button {
    padding: 6px 10px;
  }

  .segmented button.active,
  .depth-control button.active {
    color: #e9fff6;
    background: #29483d;
  }

  .search-box {
    width: min(300px, 32vw);
    height: 32px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 7px 0 10px;
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text-muted);
    background: #0e1115;
  }

  .search-box:focus-within {
    border-color: #39745f;
    box-shadow: 0 0 0 2px rgba(88, 214, 166, 0.08);
  }

  .search-box input {
    width: 100%;
    min-width: 0;
    border: 0;
    outline: 0;
    color: var(--text-primary);
    background: transparent;
    font-size: 12px;
  }

  .search-box input::placeholder {
    color: #5d6570;
  }

  .match-count {
    color: var(--accent);
    font-size: 10px;
  }

  .clear-search {
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 4px;
    color: var(--text-muted);
    background: transparent;
    cursor: pointer;
  }

  .clear-search:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
  }

  .toolbar-spacer {
    flex: 1;
  }

  .depth-control {
    display: flex;
    align-items: center;
    gap: 2px;
    color: var(--text-muted);
    font-size: 10px;
    white-space: nowrap;
  }

  .depth-control span {
    margin-right: 3px;
  }

  .depth-control button {
    min-width: 24px;
    padding: 5px 6px;
  }

  .depth-control button:hover,
  .toolbar-button:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
  }

  .toolbar-button {
    padding: 7px 10px;
    border: 1px solid var(--border);
    white-space: nowrap;
  }

  .viewer {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    background:
      linear-gradient(rgba(255, 255, 255, 0.012) 1px, transparent 1px) 0 0 / 100% 28px,
      #101318;
  }

  .tree-view {
    min-width: 100%;
    width: max-content;
    padding: 10px 8px 28px;
    font: 12px/1.5 ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  .text-view {
    min-width: max-content;
    min-height: 100%;
    margin: 0;
    padding: 16px 20px 40px;
    color: #cbd2db;
    tab-size: 2;
    white-space: pre;
    font: 12px/1.65 ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  .status-bar {
    flex: 0 0 30px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 0 14px;
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    background: #11151a;
    font-size: 10px;
  }

  .status-left {
    min-width: 0;
    display: flex;
    align-items: center;
    overflow: hidden;
    white-space: nowrap;
  }

  .success-dot {
    width: 6px;
    height: 6px;
    margin-right: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 8px rgba(88, 214, 166, 0.7);
  }

  .separator {
    padding: 0 6px;
    color: #3e4650;
  }

  .privacy {
    color: #718077;
    white-space: nowrap;
  }

  .privacy span {
    color: #4b9e7c;
    font-size: 7px;
  }

  .empty-state {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    overflow: auto;
    padding: clamp(32px, 8vh, 76px) 24px 30px;
    text-align: center;
  }

  .error-state {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 18px;
    background:
      radial-gradient(circle at 8% 0%, rgba(140, 79, 42, 0.08), transparent 28%),
      var(--surface);
  }

  .error-header {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    margin-bottom: 12px;
  }

  .error-title {
    min-width: 0;
    display: flex;
    align-items: flex-start;
    gap: 11px;
  }

  .error-title h2,
  .error-title p {
    margin: 0;
  }

  .error-title h2 {
    color: var(--text-primary);
    font-size: 15px;
  }

  .error-title p {
    max-width: 620px;
    margin-top: 4px;
    overflow: hidden;
    color: #c8a486;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 10px/1.5 ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .error-actions {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .secondary-action,
  .repair-button {
    min-height: 34px;
    border-radius: 7px;
    cursor: pointer;
    white-space: nowrap;
    font-size: 10px;
  }

  .secondary-action {
    display: inline-flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    padding: 4px 9px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    background: var(--surface-raised);
  }

  .secondary-action span {
    color: var(--text-muted);
    font-size: 8px;
  }

  .secondary-action:hover {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .repair-button {
    padding: 0 12px;
    border: 1px solid rgba(229, 153, 89, 0.42);
    color: #ffe0c6;
    background: linear-gradient(180deg, #86512f, #6f4027);
  }

  .repair-button:hover {
    border-color: #dc9562;
    background: linear-gradient(180deg, #945a35, #78462b);
  }

  .repair-button:disabled {
    cursor: wait;
    opacity: 0.65;
  }

  .repair-error {
    display: flex;
    gap: 9px;
    margin-bottom: 10px;
    padding: 8px 10px;
    border: 1px solid rgba(208, 91, 91, 0.28);
    border-radius: 7px;
    color: #d79a9a;
    background: rgba(120, 36, 36, 0.1);
    font: 10px/1.5 ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .raw-problem-shell {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid #3a3030;
    border-radius: 8px;
    background: #0c0f12;
  }

  .raw-problem-toolbar {
    flex: 0 0 34px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 11px;
    border-bottom: 1px solid #332d2d;
    color: var(--text-muted);
    background: #151719;
    font-size: 9px;
  }

  .raw-problem-text {
    flex: 1;
    width: 100%;
    min-height: 0;
    resize: none;
    border: 0;
    padding: 14px 16px 40px;
    outline: 0;
    color: #d1c7c2;
    caret-color: #ef9c70;
    background:
      linear-gradient(rgba(255, 255, 255, 0.012) 1px, transparent 1px) 0 0 / 100% 24px,
      #0c0f12;
    tab-size: 2;
    white-space: pre;
    font: 11px/1.6 ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  }

  .raw-problem-text::selection {
    color: #fff;
    background: rgba(222, 112, 74, 0.58);
  }

  .error-footer {
    flex: 0 0 36px;
    display: flex;
    align-items: end;
    justify-content: space-between;
    color: var(--text-muted);
    font-size: 9px;
  }

  .empty-visual {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 26px;
    color: #3f7965;
  }

  .brace {
    color: var(--accent);
    opacity: 0.8;
    font: 300 74px/1 ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .json-lines {
    width: 78px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .json-lines i {
    height: 5px;
    border-radius: 4px;
    background: #345e50;
  }

  .json-lines i:nth-child(1) {
    width: 66%;
  }

  .json-lines i:nth-child(2) {
    width: 100%;
    background: #477d69;
  }

  .json-lines i:nth-child(3) {
    width: 46%;
  }

  .empty-state h2 {
    margin: 0 0 10px;
    color: var(--text-primary);
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .empty-description {
    max-width: 520px;
    margin: 0 0 24px;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.7;
  }

  .primary-action {
    display: inline-flex;
    align-items: center;
    gap: 16px;
    padding: 10px 12px 10px 18px;
    font-size: 13px;
  }

  .manual-toggle {
    margin-top: 14px;
    border: 0;
    color: var(--text-muted);
    background: transparent;
    cursor: pointer;
    font-size: 11px;
  }

  .manual-toggle:hover {
    color: var(--text-secondary);
  }

  .manual-input {
    width: min(680px, 90vw);
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }

  .manual-input textarea {
    flex: 1;
    min-height: 110px;
    resize: vertical;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    outline: 0;
    color: var(--text-primary);
    background: #0b0e11;
    font: 11px/1.55 ui-monospace, SFMono-Regular, Menlo, monospace;
  }

  .manual-input button {
    align-self: stretch;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    padding: 0 14px;
    color: var(--text-primary);
    background: var(--surface-raised);
    cursor: pointer;
  }

  .manual-input button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .problem-card {
    max-width: 680px;
    display: flex;
    gap: 12px;
    margin-bottom: 20px;
    padding: 14px 18px;
    border: 1px solid rgba(229, 153, 89, 0.28);
    border-radius: 10px;
    text-align: left;
    background: rgba(134, 81, 37, 0.11);
  }

  .problem-card h2 {
    margin-bottom: 5px;
    font-size: 15px;
  }

  .problem-card p {
    margin: 2px 0;
    color: #c9a985;
    font: 11px/1.5 ui-monospace, SFMono-Regular, Menlo, monospace;
    overflow-wrap: anywhere;
  }

  .problem-icon {
    width: 22px;
    height: 22px;
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    border: 1px solid #b57947;
    border-radius: 50%;
    color: #efae76;
    font-weight: 700;
  }

  .feature-pills {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 7px;
    margin-top: 30px;
  }

  .feature-pills span {
    padding: 5px 9px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: #69737f;
    background: rgba(255, 255, 255, 0.015);
    font-size: 10px;
  }

  .toast {
    position: fixed;
    left: 50%;
    bottom: 44px;
    z-index: 20;
    transform: translateX(-50%);
    padding: 8px 13px;
    border: 1px solid #39745f;
    border-radius: 7px;
    color: #dff9ef;
    background: #1a342b;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.35);
    font-size: 11px;
  }

  @media (max-width: 820px) {
    .depth-control {
      display: none;
    }

    .search-box {
      width: min(260px, 38vw);
    }

    .shortcut-hint kbd {
      display: none;
    }

    .error-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .error-actions {
      width: 100%;
      overflow-x: auto;
    }
  }
</style>
