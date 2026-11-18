<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import VirtualList from '../../lib/ui/VirtualList.svelte';
  import { stIpc, type StNode } from '../../lib/ipc';
  import {
    cellText,
    COLUMNS,
    defaultColumnState,
    driveAccounting,
    formatSlack,
    formatUnitBytes,
    moveColumn,
    parseColumnState,
    visibleColumns,
    type ColumnState,
  } from './logic';
  import { storage } from './store.svelte';

  const KEY = 'nomnom.storage.columns';
  let cols = $state<ColumnState>(defaultColumnState());
  try {
    cols = parseColumnState(localStorage.getItem(KEY));
  } catch {
    /* storage can be unavailable */
  }
  function saveCols() {
    try {
      localStorage.setItem(KEY, JSON.stringify(cols));
    } catch {
      /* storage can be unavailable */
    }
  }

  const shown = $derived(visibleColumns(cols));
  const template = $derived(shown.map((c) => `${c.w}px`).join(' '));
  const totalWidth = $derived(shown.reduce((n, c) => n + c.w, 0));

  interface FlatRow {
    key: string;
    node: StNode;
    depth: number;
    kind: 'node' | 'files_group' | 'virtual';
    groupParent?: number;
    expandable: boolean;
    expanded: boolean;
  }

  let expandedDirs = $state(new Set<number>());
  let expandedFiles = $state(new Set<number>());
  let groupLooseFiles = $state(true);
  let showDriveSpace = $state(true);
  let dirKids = $state(new Map<number, StNode[]>());
  let fileKids = $state(new Map<number, StNode[]>());
  let menuOpen = $state(false);
  let cursor = $state(-1);
  let list = $state<ReturnType<typeof VirtualList<FlatRow>> | null>(null);
  let dragKey = $state<string | null>(null);
  let loadSeq = 0;

  // Keep the root being viewed expanded by default whenever the user navigates to a folder.
  let prevRoot = -1;
  $effect(() => {
    const cur = storage.current;
    if (cur !== prevRoot) {
      prevRoot = cur;
      expandedDirs = new Set([cur]);
      expandedFiles = new Set();
    }
  });

  async function refreshLoaded() {
    const id = storage.sessionId;
    if (id == null) return;
    const seq = ++loadSeq;
    const dirsToFetch = [...expandedDirs];
    const filesToFetch = [...expandedFiles];
    const text = storage.text || undefined;

    const dirEntries = await Promise.all(
      dirsToFetch.map(async (nid) => {
        if (groupLooseFiles) {
          const p = await stIpc.children(id, nid, storage.sort, storage.desc, 0, 1500, text, 'dirs').catch(() => ({ total: 0, items: [] }));
          storage.registerNodes(p.items);
          return [nid, p.items] as const;
        } else {
          const p = await stIpc.children(id, nid, storage.sort, storage.desc, 0, 2000, text).catch(() => ({ total: 0, items: [] }));
          storage.registerNodes(p.items);
          return [nid, p.items] as const;
        }
      }),
    );
    const fileEntries = await Promise.all(
      filesToFetch.map(async (nid) => {
        const p = await stIpc.children(id, nid, storage.sort, storage.desc, 0, 2000, text, 'files').catch(() => ({ total: 0, items: [] }));
        storage.registerNodes(p.items);
        return [nid, p.items] as const;
      }),
    );
    if (seq !== loadSeq) return;
    dirKids = new Map(dirEntries);
    fileKids = new Map(fileEntries);
  }

  $effect(() => {
    void [storage.sessionId, storage.version, storage.sort, storage.desc, storage.text, groupLooseFiles, expandedDirs, expandedFiles];
    void refreshLoaded();
  });

  async function toggleDir(nid: number) {
    const next = new Set(expandedDirs);
    if (next.has(nid)) next.delete(nid);
    else next.add(nid);
    expandedDirs = next;
  }

  function toggleFilesGroup(parentId: number) {
    const next = new Set(expandedFiles);
    if (next.has(parentId)) next.delete(parentId);
    else next.add(parentId);
    expandedFiles = next;
  }

  async function expandToLevel(maxLevel: number) {
    const id = storage.sessionId;
    if (id == null) return;
    if (maxLevel <= 0) {
      expandedDirs = new Set();
      expandedFiles = new Set();
      return;
    }
    const next = new Set<number>([storage.current]);
    let frontier = [storage.current];
    for (let lvl = 1; lvl < maxLevel && frontier.length > 0; lvl++) {
      const nextFrontier: number[] = [];
      const pages = await Promise.all(
        frontier.slice(0, 60).map((nid) =>
          stIpc.children(id, nid, 'size', true, 0, 80, undefined, 'dirs').catch(() => ({ total: 0, items: [] })),
        ),
      );
      for (const p of pages) {
        storage.registerNodes(p.items);
        for (const child of p.items) {
          if (child.is_dir && !child.is_link && (child.dirs > 0 || (child.own_files ?? 0) > 0)) {
            next.add(child.id);
            if (next.size < 250) nextFrontier.push(child.id);
          }
        }
      }
      frontier = nextFrontier;
    }
    expandedDirs = next;
  }

  const flatRows = $derived.by((): FlatRow[] => {
    const rootNode = storage.detail?.node;
    if (!rootNode) return [];
    const out: FlatRow[] = [];
    const acct = showDriveSpace && storage.current === 0 ? driveAccounting(storage.progress, storage.wholeDrive) : null;
    const denom = acct ? Math.max(1, acct.totalBytes) : Math.max(1, rootNode.size);

    const pushFolderChildren = (parent: StNode, depth: number) => {
      const kids = dirKids.get(parent.id) ?? [];
      const ownFiles = parent.own_files ?? 0;
      const ownSize = parent.own_size ?? 0;
      const ownAlloc = parent.own_alloc ?? 0;

      if (groupLooseFiles && ownFiles > 0) {
        const filesOpen = expandedFiles.has(parent.id);
        const pct = parent.size > 0 ? (ownSize / parent.size) * 100 : 0;
        const groupNode: StNode = {
          id: -1000 - parent.id,
          name: `[${ownFiles.toLocaleString()} ${ownFiles === 1 ? 'file' : 'files'}]`,
          is_dir: false,
          is_link: false,
          size: ownSize,
          alloc: ownAlloc,
          own_size: ownSize,
          own_alloc: ownAlloc,
          own_files: ownFiles,
          files: ownFiles,
          dirs: 0,
          mtime: parent.mtime,
          ctime: null,
          atime: null,
          attrs: 0,
          flags: 0,
          percent: pct,
          parent: parent.id,
        };

        // Insert `[Files]` sorted among subfolders when sorted by size/alloc, or first by name.
        let inserted = false;
        const groupRow: FlatRow = {
          key: `fg-${parent.id}`,
          node: groupNode,
          depth,
          kind: 'files_group',
          groupParent: parent.id,
          expandable: true,
          expanded: filesOpen,
        };
        const emitGroup = () => {
          out.push(groupRow);
          if (filesOpen) {
            for (const f of fileKids.get(parent.id) ?? []) {
              out.push({ key: `n-${f.id}`, node: f, depth: depth + 1, kind: 'node', expandable: false, expanded: false });
            }
          }
          inserted = true;
        };

        for (const k of kids) {
          if (!inserted && storage.desc && (storage.sort === 'size' || storage.sort === 'percent') && ownSize >= k.size) {
            emitGroup();
          }
          const canExpand = k.is_dir && !k.is_link && (k.dirs > 0 || (k.own_files ?? 0) > 0 || k.files > 0);
          const isOpen = canExpand && expandedDirs.has(k.id);
          out.push({ key: `n-${k.id}`, node: k, depth, kind: 'node', expandable: canExpand, expanded: isOpen });
          if (isOpen) pushFolderChildren(k, depth + 1);
        }
        if (!inserted) emitGroup();
      } else {
        for (const k of kids) {
          const canExpand = k.is_dir && !k.is_link && (k.dirs > 0 || (k.own_files ?? 0) > 0 || k.files > 0);
          const isOpen = canExpand && expandedDirs.has(k.id);
          out.push({ key: `n-${k.id}`, node: k, depth, kind: 'node', expandable: canExpand, expanded: isOpen });
          if (isOpen) pushFolderChildren(k, depth + 1);
        }
      }
    };

    const rootExpandable = rootNode.dirs > 0 || rootNode.files > 0;
    const rootOpen = expandedDirs.has(rootNode.id);
    const rootAdjusted: StNode = acct
      ? { ...rootNode, percent: (rootNode.alloc / denom) * 100 }
      : { ...rootNode, percent: 100 };
    out.push({
      key: `n-${rootNode.id}`,
      node: rootAdjusted,
      depth: 0,
      kind: 'node',
      expandable: rootExpandable,
      expanded: rootOpen,
    });
    if (rootOpen) pushFolderChildren(rootNode, 1);

    if (acct) {
      if (acct.systemBytes > 0) {
        out.push({
          key: 'virt-system',
          node: {
            id: -2,
            name: '[System and unreadable space]',
            is_dir: false,
            is_link: false,
            size: acct.systemBytes,
            alloc: acct.systemBytes,
            files: 0,
            dirs: 0,
            mtime: null,
            ctime: null,
            atime: null,
            attrs: 0x4,
            flags: 0,
            percent: (acct.systemBytes / denom) * 100,
            parent: null,
          },
          depth: 0,
          kind: 'virtual',
          expandable: false,
          expanded: false,
        });
      }
      if (acct.freeBytes > 0) {
        out.push({
          key: 'virt-free',
          node: {
            id: -3,
            name: '[Free space]',
            is_dir: false,
            is_link: false,
            size: acct.freeBytes,
            alloc: acct.freeBytes,
            files: 0,
            dirs: 0,
            mtime: null,
            ctime: null,
            atime: null,
            attrs: 0,
            flags: 0,
            percent: (acct.freeBytes / denom) * 100,
            parent: null,
          },
          depth: 0,
          kind: 'virtual',
          expandable: false,
          expanded: false,
        });
      }
    }
    return out;
  });

  function startResize(e: PointerEvent, key: string, startW: number) {
    e.preventDefault();
    e.stopPropagation();
    const x0 = e.clientX;
    const move = (ev: PointerEvent) => {
      cols.widths = { ...cols.widths, [key]: Math.max(50, Math.round(startW + ev.clientX - x0)) };
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      saveCols();
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  function toggleColumn(key: string) {
    cols.hidden = cols.hidden.includes(key) ? cols.hidden.filter((k) => k !== key) : [...cols.hidden, key];
    saveCols();
  }

  function drop(before: string) {
    if (dragKey) {
      cols.order = moveColumn(cols.order, dragKey, before);
      saveCols();
    }
    dragKey = null;
  }

  function activateRow(r: FlatRow) {
    if (r.kind === 'files_group' && r.groupParent != null) {
      toggleFilesGroup(r.groupParent);
      return;
    }
    if (r.kind === 'virtual') return;
    if (r.node.is_dir && !r.node.is_link) {
      if (r.node.id === storage.current) void toggleDir(r.node.id);
      else void storage.go(r.node.id);
    } else if (!r.node.is_dir && r.node.id >= 0) {
      storage.selected = new Set([r.node.id]);
      void storage.openSelected(false);
    }
  }

  function context(e: MouseEvent, i: number) {
    const r = flatRows[i];
    if (!r || r.kind !== 'node' || r.node.id < 0) return;
    e.preventDefault();
    cursor = i;
    const ids = flatRows.map((x) => x.node.id);
    if (!storage.selected.has(r.node.id)) storage.clickRow(i, { ctrl: false, shift: false }, ids);
    storage.menu = { x: e.clientX, y: e.clientY };
  }

  function onKey(e: KeyboardEvent) {
    const n = flatRows.length;
    if (e.target instanceof HTMLInputElement || n === 0) return;
    const ids = flatRows.map((x) => x.node.id);
    const mods = { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey };
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      const next = e.key === 'Home' ? 0 : e.key === 'End' ? n - 1 : Math.min(n - 1, Math.max(0, (cursor < 0 ? (e.key === 'ArrowDown' ? -1 : n) : cursor) + (e.key === 'ArrowDown' ? 1 : -1)));
      cursor = next;
      storage.clickRow(next, { ctrl: false, shift: mods.shift }, ids);
      list?.reveal(next);
    } else if (e.key === 'ArrowRight' && cursor >= 0) {
      e.preventDefault();
      const cur = flatRows[cursor];
      if (cur.kind === 'files_group' && cur.groupParent != null && !cur.expanded) toggleFilesGroup(cur.groupParent);
      else if (cur.expandable && !cur.expanded) void toggleDir(cur.node.id);
    } else if (e.key === 'ArrowLeft' && cursor >= 0) {
      e.preventDefault();
      const cur = flatRows[cursor];
      if (cur.kind === 'files_group' && cur.groupParent != null && cur.expanded) toggleFilesGroup(cur.groupParent);
      else if (cur.expandable && cur.expanded) void toggleDir(cur.node.id);
      else if (cur.node.parent != null) {
        const pIdx = flatRows.findIndex((r) => r.node.id === cur.node.parent);
        if (pIdx >= 0) {
          cursor = pIdx;
          storage.clickRow(pIdx, { ctrl: false, shift: false }, ids);
          list?.reveal(pIdx);
        }
      }
    } else if (e.key === 'Enter' && cursor >= 0) {
      e.preventDefault();
      activateRow(flatRows[cursor]);
    } else if (e.key === 'Backspace') {
      e.preventDefault();
      void storage.up();
    } else if ((e.key === 'a' || e.key === 'A') && mods.ctrl) {
      e.preventDefault();
      storage.selectAll(ids);
    } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      if (cursor < 0) cursor = 0;
      const cur = flatRows[cursor];
      if (cur.kind !== 'node' || cur.node.id < 0) return;
      if (!storage.selected.has(cur.node.id)) storage.clickRow(cursor, { ctrl: false, shift: false }, ids);
      const where = (e.currentTarget as HTMLElement).querySelector('.tr.cur') ?? e.currentTarget;
      const rect = (where as HTMLElement).getBoundingClientRect();
      storage.menu = { x: Math.round(rect.left + 24), y: Math.round(rect.bottom) };
    }
  }
</script>

<div class="treegrid" role="presentation">
  <div class="subbar">
    <span class="lvl-group" role="group" aria-label="Expand tree levels">
      <span class="hint">Expand:</span>
      <button class="tiny" onclick={() => expandToLevel(0)}>Collapse</button>
      <button class="tiny" onclick={() => expandToLevel(1)}>Level 1</button>
      <button class="tiny" onclick={() => expandToLevel(2)}>Level 2</button>
      <button class="tiny" onclick={() => expandToLevel(3)}>Level 3</button>
    </span>
    <label class="chk"><input type="checkbox" bind:checked={groupLooseFiles} /> Group loose [files]</label>
    {#if storage.wholeDrive && storage.current === 0}
      <label class="chk"><input type="checkbox" bind:checked={showDriveSpace} /> Show free and system space</label>
    {/if}
  </div>

  <div class="scroll">
    <div class="header" style:width="{totalWidth}px" style:grid-template-columns={template} role="row">
      {#each shown as c (c.key)}
        <div
          class="th"
          class:right={c.align === 'right'}
          role="columnheader"
          tabindex="-1"
          aria-sort={storage.sort === c.sort ? (storage.desc ? 'descending' : 'ascending') : 'none'}
          draggable="true"
          ondragstart={() => (dragKey = c.key)}
          ondragover={(e) => e.preventDefault()}
          ondrop={() => drop(c.key)}
        >
          <button class="sortbtn" disabled={!c.sort} onclick={() => c.sort && storage.setSort(c.sort)}>
            {c.label}{#if storage.sort === c.sort}<Icon name={storage.desc ? 'arrow-down' : 'arrow-up'} size={12} stroke={2.6} />{/if}
          </button>
          <span class="grip" role="separator" aria-label={`Resize ${c.label}`} onpointerdown={(e) => startResize(e, c.key, c.w)}></span>
        </div>
      {/each}
    </div>

    <div class="body" style:min-width="{totalWidth}px">
      {#if flatRows.length === 0}
        <p class="empty">{storage.busy ? 'Looking…' : 'Nothing to show here.'}</p>
      {:else}
        <VirtualList bind:this={list} items={flatRows} label="Folder hierarchy" onkeydown={onKey}>
          {#snippet row(r: FlatRow, i: number)}
            {@const n = r.node}
            <div
              class="tr"
              class:sel={n.id >= 0 && storage.selected.has(n.id)}
              class:cur={i === cursor}
              class:virt={r.kind !== 'node'}
              style:grid-template-columns={template}
              style:width="{totalWidth}px"
              role="option"
              aria-selected={n.id >= 0 && storage.selected.has(n.id)}
              tabindex="-1"
              onclick={(e) => {
                cursor = i;
                if (r.kind === 'files_group' && r.groupParent != null) {
                  toggleFilesGroup(r.groupParent);
                } else if (n.id >= 0) {
                  storage.clickRow(i, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey }, flatRows.map((x) => x.node.id));
                }
              }}
              ondblclick={() => activateRow(r)}
              oncontextmenu={(e) => context(e, i)}
              onkeydown={() => {}}
            >
              {#each shown as c (c.key)}
                {#if c.key === 'name'}
                  <div class="td name" style:padding-left="{8 + r.depth * 16}px" title={n.name}>
                    {#if r.expandable}
                      <button
                        class="twist"
                        aria-label={r.expanded ? `Collapse ${n.name}` : `Expand ${n.name}`}
                        aria-expanded={r.expanded}
                        onclick={(e) => {
                          e.stopPropagation();
                          if (r.kind === 'files_group' && r.groupParent != null) toggleFilesGroup(r.groupParent);
                          else void toggleDir(n.id);
                        }}
                      >
                        {r.expanded ? '▾' : '▸'}
                      </button>
                    {:else}
                      <span class="twist"></span>
                    {/if}
                    <Icon name={n.is_dir || r.kind === 'files_group' ? 'folder' : 'file'} size={14} />
                    <span class="text" translate="no">{n.name}</span>
                    {#if n.is_link}<span class="tag">link</span>{/if}
                    {#if n.flags & 64}<span class="tag ok" title="NTFS compressed">compressed</span>{/if}
                    {#if n.flags & 128}<span class="tag" title="Has NTFS alternate data streams">streams</span>{/if}
                    {#if n.flags & 16}<span class="tag warn" title="Some of this folder couldn't be read">partial</span>{/if}
                    {#if n.flags & 4}<span class="tag" title="Another name for data already counted">same data</span>{/if}
                  </div>
                {:else if c.key === 'percent'}
                  <div class="td pct">
                    <div class="bar"><div class:free={n.id === -3} class:sys={n.id === -2} style:width="{Math.min(100, n.percent)}%"></div></div>
                    <span class="tabular">{n.percent < 0.1 && n.percent > 0 ? '<0.1' : n.percent.toFixed(1)}%</span>
                  </div>
                {:else if c.key === 'wasted'}
                  {@const sl = formatSlack(n.size, n.alloc, storage.unit)}
                  <div class="td tabular right" class:saved={sl.kind === 'saved'}>{r.kind === 'virtual' ? '' : sl.text}</div>
                {:else if c.key === 'path_len'}
                  <div class="td tabular right" class:warnpath={(n.path_len ?? 0) >= 260}>{cellText(c.key, n, storage.unit)}</div>
                {:else}
                  <div class="td tabular" class:right={c.align === 'right'}>
                    {r.kind === 'files_group' && c.key === 'files' ? n.files.toLocaleString() : cellText(c.key, n, storage.unit)}
                  </div>
                {/if}
              {/each}
            </div>
          {/snippet}
        </VirtualList>
      {/if}
    </div>
  </div>

  <div class="foot">
    <button class="link" onclick={() => (menuOpen = !menuOpen)} aria-expanded={menuOpen}>Columns ({shown.length}/{COLUMNS.length})</button>
    <span class="hint">Right arrow expands a folder, double-click opens it inside the view.</span>
    {#if menuOpen}
      <div class="menu" role="group" aria-label="Choose columns">
        {#each COLUMNS as c (c.key)}
          <label><input type="checkbox" checked={c.key === 'name' || !cols.hidden.includes(c.key)} disabled={c.key === 'name'} onchange={() => toggleColumn(c.key)} /> {c.label}</label>
        {/each}
        <button class="link" onclick={() => { cols = defaultColumnState(); saveCols(); }}>Reset columns</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .treegrid { display: grid; grid-template-rows: auto 1fr auto; height: 100%; min-height: 0; position: relative; }
  .subbar { display: flex; flex-wrap: wrap; gap: 8px 14px; align-items: center; padding: 5px 8px; border-bottom: 1px solid var(--border); background: var(--surface-1); font-size: 12.5px; }
  .lvl-group { display: inline-flex; gap: 4px; align-items: center; }
  .tiny { padding: 2px 8px; font-size: 12px; border-radius: 6px; border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  .tiny:hover { background: var(--surface-3); }
  .chk { display: inline-flex; gap: 5px; align-items: center; color: var(--text-2); cursor: pointer; }
  .scroll { display: grid; grid-template-rows: auto 1fr; min-height: 0; overflow-x: auto; }
  .header { display: grid; position: sticky; top: 0; background: var(--surface-1); border-bottom: 1px solid var(--border); z-index: 2; }
  .th { position: relative; display: flex; align-items: center; padding: 0 4px; }
  .th.right { justify-content: flex-end; }
  .sortbtn { border: 0; background: none; color: var(--text-2); font-weight: 600; padding: 6px 4px; cursor: pointer; white-space: nowrap; }
  .sortbtn:disabled { cursor: default; }
  .grip { position: absolute; right: 0; top: 4px; bottom: 4px; width: 6px; cursor: col-resize; border-right: 1px solid var(--border-strong); }
  .body { min-height: 0; }
  .tr { display: grid; align-items: center; height: 100%; cursor: default; border-radius: 4px; }
  .tr:hover { background: var(--surface-2); }
  .tr.sel { background: var(--surface-3); }
  .tr.cur { outline: 1px dotted var(--text-3); outline-offset: -1px; }
  .tr.virt { color: var(--text-2); font-style: italic; }
  .td { padding: 0 8px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .td.right { text-align: right; }
  .td.saved { color: var(--ok); }
  .td.warnpath { color: var(--danger); font-weight: 600; }
  .name { display: flex; gap: 5px; align-items: center; user-select: text; }
  .twist { width: 16px; height: 18px; flex: none; border: 0; background: none; color: var(--text-2); cursor: pointer; padding: 0; font-size: 11px; }
  .text { overflow: hidden; text-overflow: ellipsis; }
  .tag { font-size: 11px; color: var(--info); border: 1px solid var(--border-strong); border-radius: 8px; padding: 0 6px; flex: none; font-style: normal; }
  .tag.ok { color: var(--ok); }
  .tag.warn { color: var(--danger); }
  .pct { display: flex; gap: 8px; align-items: center; }
  .bar { flex: 1; height: 6px; border-radius: 3px; background: var(--surface-3); overflow: hidden; }
  .bar div { height: 100%; background: var(--accent); }
  .bar div.free { background: var(--ok); opacity: 0.7; }
  .bar div.sys { background: var(--text-3); }
  .pct span { width: 44px; text-align: right; color: var(--text-2); font-size: 12px; font-style: normal; }
  .empty { color: var(--text-3); padding: 16px; margin: 0; }
  .foot { display: flex; gap: 14px; align-items: center; padding: 6px 8px; border-top: 1px solid var(--border); position: relative; }
  .hint { color: var(--text-3); font-size: 12.5px; }
  .link { border: 0; background: none; color: var(--accent-text); text-decoration: underline; cursor: pointer; padding: 0; }
  .menu { position: absolute; bottom: 100%; left: 4px; background: var(--surface-2); border: 1px solid var(--border-strong); border-radius: var(--radius-s); padding: 8px 12px; display: grid; grid-template-columns: repeat(2, auto); gap: 4px 16px; z-index: 5; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35); }
  .menu label { display: flex; gap: 8px; align-items: center; white-space: nowrap; }
</style>
