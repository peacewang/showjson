<script lang="ts">
  import {
    compactJson,
    getJsonType,
    isJsonContainer,
    pathKey,
    pathToJsonPath,
    valuePreview,
    type SearchMatches,
  } from "$lib/json/format";
  import type { JsonValue } from "$lib/json/types";

  export let value: JsonValue;
  export let name: string;
  export let path: Array<string | number>;
  export let depth = 0;
  export let expansionLevel = 2;
  export let expansionVersion = 0;
  export let matches: SearchMatches;
  export let query = "";
  export let onCopy: (text: string, label: string) => void;

  let expanded = depth < expansionLevel;
  let previousExpansionVersion = expansionVersion;

  $: if (previousExpansionVersion !== expansionVersion) {
    previousExpansionVersion = expansionVersion;
    expanded = depth < expansionLevel;
  }

  $: container = isJsonContainer(value);
  $: entries = Array.isArray(value)
    ? value.map((child, index) => [String(index), child] as const)
    : container
      ? Object.entries(value as Record<string, JsonValue>)
      : [];
  $: currentPathKey = pathKey(path);
  $: directMatch = matches.direct.has(currentPathKey);
  $: branchMatch = matches.branches.has(currentPathKey);
  $: if (query && branchMatch) expanded = true;
  $: type = getJsonType(value);
  $: preview = valuePreview(value);
  $: pathLabel = pathToJsonPath(path);

  function copyValue(event: MouseEvent) {
    event.stopPropagation();
    onCopy(typeof value === "string" ? value : compactJson(value), "值");
  }

  function copyPath(event: MouseEvent) {
    event.stopPropagation();
    onCopy(pathLabel, "JSONPath");
  }
</script>

<div class:search-branch={query && branchMatch} class="node" data-depth={depth}>
  <div
    class:match={directMatch}
    class="node-row"
  >
    <span class="indent" style={`--depth: ${depth}`}></span>
    <button
      class:open={expanded}
      class:invisible={!container}
      class="chevron"
      aria-label={expanded ? "折叠节点" : "展开节点"}
      disabled={!container}
      onclick={() => (expanded = !expanded)}
    >›</button>
    <span class="key">{name}</span>
    <span class="punctuation">:</span>

    {#if container}
      <span class="summary">{preview}</span>
    {:else}
      <span class={`value value-${type}`} title={preview}>{preview}</span>
    {/if}

    <span class="row-actions">
      <button aria-label="复制 JSONPath" onclick={copyPath} title={`复制 ${pathLabel}`}>Path</button>
      <button aria-label="复制值" onclick={copyValue} title="复制值">Copy</button>
    </span>
  </div>

  {#if container && expanded}
    <div class="children">
      {#each entries as [childName, childValue]}
        <svelte:self
          value={childValue}
          name={Array.isArray(value) ? `[${childName}]` : childName}
          path={[...path, Array.isArray(value) ? Number(childName) : childName]}
          depth={depth + 1}
          {expansionLevel}
          {expansionVersion}
          {matches}
          {query}
          {onCopy}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .node {
    min-width: max-content;
  }

  .node-row {
    min-height: 28px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 12px;
    border-radius: 5px;
    color: var(--text-primary);
    white-space: nowrap;
  }

  .node-row:hover {
    background: var(--surface-hover);
  }

  .node-row.match {
    background: var(--search-bg);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .indent {
    display: inline-block;
    width: calc(var(--depth) * 20px + 8px);
    flex: 0 0 auto;
  }

  .chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    color: var(--text-muted);
    font-size: 18px;
    transform: rotate(0deg);
    transition: transform 100ms ease;
    border: 0;
    padding: 0;
    background: transparent;
    cursor: pointer;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .chevron.invisible {
    visibility: hidden;
  }

  .key {
    color: var(--json-key);
  }

  .punctuation,
  .summary {
    color: var(--text-muted);
  }

  .value {
    display: inline-block;
    max-width: min(720px, 58vw);
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .value-string {
    color: var(--json-string);
  }

  .value-number {
    color: var(--json-number);
  }

  .value-boolean {
    color: var(--json-boolean);
  }

  .value-null {
    color: var(--json-null);
    font-style: italic;
  }

  .row-actions {
    position: sticky;
    right: 8px;
    margin-left: auto;
    display: inline-flex;
    gap: 4px;
    opacity: 0;
    padding-left: 18px;
    background: linear-gradient(90deg, transparent, var(--surface-hover) 18px);
  }

  .node-row:hover .row-actions,
  .node-row:focus-within .row-actions {
    opacity: 1;
  }

  .row-actions button {
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 2px 6px;
    color: var(--text-secondary);
    background: var(--surface-raised);
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }

  .row-actions button:hover {
    color: var(--text-primary);
    border-color: var(--border-strong);
  }
</style>
