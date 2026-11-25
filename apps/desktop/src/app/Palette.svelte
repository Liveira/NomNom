<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { actionsIpc, clipIpc, fileIpc, ipc, launcherIpc, noteIpc, ocrIpc, reminderIpc, snippetIpc, type CustomAction, type FileHit, type LauncherEntry, type ReminderPreview, type Snippet } from '../lib/ipc';
  import { calc, lastTwo, rank, showNumber } from '../lib/palette';
  import { convert } from '../lib/units';
  import { TOOLS } from '../lib/tools';
  import { dueText, noteFrom, startsLikeReminder } from '../modules/reminders/logic';
  import { parentOf } from '../modules/rules/logic';
  import { THEMES_IN_ORDER } from '../modules/settings/themes';
  import { app } from './state.svelte';

  /**
   * The command palette (Ctrl+K inside NomNom) and the launcher (a small window of its own,
   * Alt+Space by default) are the same search box. `floating` is the launcher.
   */
  let { floating = false, onclose }: { floating?: boolean; onclose: () => void } = $props();

  interface Item {
    key: string;
    title: string;
    hint: string;
    icon: string;
    keywords?: string;
    run: () => void | Promise<void>;
    /** Ctrl+Enter: for a file, show it in its folder. */
    alt?: () => void | Promise<void>;
  }

  let query = $state('');
  let active = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLUListElement | null>(null);
  let entries = $state<LauncherEntry[]>([]);
  let loadedActions = $state<CustomAction[]>([]);
  let snippets = $state<Snippet[]>([]);
  const actions = $derived(loadedActions);
  let note = $state('');
  let answer = $state<{ title: string; ok: boolean; text: string } | null>(null);

  const pages = $derived(
    floating
      ? [
          { id: 'home', name: 'Home', icon: 'home' },
          { id: 'tools', name: 'Tools', icon: 'tools' },
          ...TOOLS.map((t) => ({ id: t.id, name: t.title, icon: t.icon })),
          { id: 'changelog', name: 'Patch notes', icon: 'scroll-text' },
          { id: 'settings', name: 'Settings', icon: 'settings' },
        ]
      : app.nav.map((n) => ({ id: n.id, name: n.name, icon: n.icon })),
  );

  function close() {
    if (floating) void launcherIpc.close();
    else onclose();
  }

  function go(id: string) {
    if (floating) void launcherIpc.go(id);
    else {
      onclose();
      app.go(id);
    }
  }

  async function runAction(a: CustomAction) {
    if (a.params.length > 0) {
      // It needs values: the page for actions asks for them.
      try {
        localStorage.setItem('nomnom.pendingAction', a.id);
      } catch {
        /* the page opens without the form */
      }
      go('actions');
      // Already on that page? Tell it there is something waiting.
      window.dispatchEvent(new Event('nomnompending'));
      return;
    }
    try {
      const r = await actionsIpc.run(a.id, {});
      if (a.kind === 'command') {
        answer = { title: a.name, ok: r.ok, text: (r.output.trim() || r.message) + (r.truncated ? '\n…' : '') };
        return;
      }
      close();
    } catch (e) {
      note = String(e);
    }
  }

  const extras = $derived.by((): Item[] => {
    const out: Item[] = [];
    for (const t of THEMES_IN_ORDER) {
      out.push({
        key: `theme:${t.id}`,
        title: `Theme: ${t.name}`,
        hint: 'Change theme & layout',
        icon: t.icon,
        keywords: 'colour color appearance dark light layout tema',
        run: async () => {
          if (floating) {
            await ipc.updateSettings({ theme: t.id }).catch(() => {});
            close();
          } else {
            onclose();
            void app.setTheme(t.id);
          }
        },
      });
    }
    if (!floating) {
      for (const [code, name] of [['auto', 'Automatic'], ['en', 'English'], ['pt', 'Português'], ['es', 'Español']] as const) {
        out.push({ key: `lang:${code}`, title: `Language: ${name}`, hint: 'Change the language', icon: 'globe', keywords: 'idioma', run: () => { onclose(); void app.setLanguage(code); } });
      }
      out.push({ key: 'sidebar', title: 'Show or hide the sidebar', hint: 'Layout', icon: 'panel', run: () => { onclose(); void app.toggleSidebar(); } });
    }
    out.push({
      key: 'guide',
      title: 'Guide: How to use NomNom',
      hint: 'Complete guide & shortcuts',
      icon: 'book',
      keywords: 'help manual tutorial guia como usar',
      run: () => {
        try {
          localStorage.setItem('nomnom.openGuide', 'guide-launcher');
        } catch {
          /* ignore */
        }
        go('tools');
      },
    });
    if (floating || app.settings?.clipboard.enabled) {
      out.push({ key: 'clip', title: 'Open the clipboard history', hint: 'Action', icon: 'clipboard', run: () => { close(); void clipIpc.open(); } });
    }
    if (floating || app.settings?.text.ocr) {
      out.push({ key: 'ocr', title: 'Read text from the screen', hint: 'Action', icon: 'search', keywords: 'ocr capture', run: () => { close(); void ocrIpc.start(); } });
    }
    return out;
  });

  const all = $derived.by((): Item[] => {
    const list: Item[] = [];
    for (const a of actions.filter((x) => x.in_palette)) {
      list.push({
        key: `act:${a.id}`,
        title: a.name,
        hint: a.params.length > 0 ? 'Your action · asks for values' : 'Your action',
        icon: a.kind === 'link' ? 'link' : a.kind === 'folder' ? 'folder' : a.kind === 'command' ? 'terminal' : 'rocket',
        run: () => runAction(a),
      });
    }
    for (const p of pages) {
      list.push({
        key: `page:${p.id}`,
        title: p.name,
        hint: 'Open',
        icon: p.icon,
        keywords: p.id === 'changelog' ? 'changelog release versions history updates novidades notas versao' : undefined,
        run: () => go(p.id),
      });
    }
    for (const sn of snippets) {
      list.push({
        key: `snip:${sn.id}`,
        title: sn.name,
        hint: 'Snippet',
        icon: 'file-text',
        keywords: sn.body.slice(0, 80),
        run: async () => {
          try {
            // From the launcher the text is pasted where the person was typing; inside NomNom it is copied.
            await snippetIpc.use(sn.id, floating);
            close();
          } catch (e) {
            note = String(e);
          }
        },
      });
    }
    list.push(...extras);
    const rest: LauncherEntry[] = entries;
    for (const kind of ['recent', 'folder', 'app'] as const) {
      for (const e of rest.filter((x) => x.kind === kind)) {
        list.push({
          key: `${kind}:${e.path}`,
          title: e.name,
          hint: kind === 'app' ? 'Program' : kind === 'folder' ? 'Folder' : 'Recent file',
          icon: kind === 'app' ? 'window' : kind === 'folder' ? 'folder' : 'file',
          run: async () => {
            try {
              await launcherIpc.open(e.path);
              if (!floating) onclose();
            } catch (err) {
              note = String(err);
            }
          },
        });
      }
    }
    return list;
  });

  /** A reminder said in words: shown as a row once the words make sense. */
  let remind = $state<ReminderPreview | null>(null);
  let remindToken = 0;
  $effect(() => {
    const q = query;
    const mine = ++remindToken;
    if (!startsLikeReminder(q)) {
      remind = null;
      return;
    }
    void reminderIpc
      .preview(q)
      .then((p) => {
        if (mine === remindToken) remind = p;
      })
      .catch(() => {
        if (mine === remindToken) remind = null;
      });
  });
  const noteText = $derived(noteFrom(query));

  /** Files whose name has the words typed, from the system's own index. */
  let fileHits = $state<FileHit[]>([]);
  let fileToken = 0;
  $effect(() => {
    const q = query;
    const mine = ++fileToken;
    if (q.trim().length < 2 || startsLikeReminder(q) || noteFrom(q) !== null || calc(q) !== null) {
      fileHits = [];
      return;
    }
    const wait = setTimeout(() => {
      void fileIpc
        .search(q)
        .then((h) => {
          if (mine === fileToken) fileHits = h;
        })
        .catch(() => {
          if (mine === fileToken) fileHits = [];
        });
    }, 280);
    return () => clearTimeout(wait);
  });
  const fileRows = $derived.by((): Item[] =>
    fileHits.map((h) => ({
      key: `file:${h.path}`,
      title: h.name,
      hint: lastTwo(parentOf(h.path)),
      icon: h.is_dir ? 'folder' : 'file',
      run: async () => {
        try {
          await launcherIpc.openPath(h.path, false);
          if (!floating) onclose();
        } catch (err) {
          note = String(err);
        }
      },
      alt: async () => {
        try {
          await launcherIpc.openPath(h.path, true);
          if (!floating) onclose();
        } catch (err) {
          note = String(err);
        }
      },
    })),
  );

  const sum = $derived(calc(query));
  const results = $derived.by((): Item[] => {
    const found = rank(all, query, 40);
    const special: Item[] = [];
    if (remind) {
      const r = remind;
      special.push({
        key: 'remind',
        title: `Remind: ${r.text}`,
        hint: dueText(r.due_at, Math.floor(Date.now() / 1000)),
        icon: 'bell',
        run: async () => {
          try {
            await reminderIpc.addText(query);
            close();
          } catch (e) {
            note = String(e);
          }
        },
      });
    }
    if (noteText) {
      special.push({
        key: 'note',
        title: `Save note: ${noteText.length > 60 ? noteText.slice(0, 60) + '…' : noteText}`,
        hint: 'Notes',
        icon: 'pencil',
        run: async () => {
          try {
            await noteIpc.save(null, noteText);
            close();
          } catch (e) {
            note = String(e);
          }
        },
      });
    }
    const withFiles = [...found, ...fileRows.filter((f) => !found.some((x) => x.key === f.key))];
    const unit = convert(query);
    if (unit) {
      const answer = `${showNumber(Number(unit.value.toPrecision(8)))} ${unit.to}`;
      special.unshift({
        key: 'unit',
        title: `= ${answer}`,
        hint: 'Press Enter to copy the answer',
        icon: 'sparkle',
        run: async () => {
          try {
            await navigator.clipboard.writeText(answer);
          } catch {
            /* copying is a courtesy */
          }
          close();
        },
      });
    }
    if (sum === null) return [...special, ...withFiles];
    const text = showNumber(sum);
    const row: Item = {
      key: 'calc',
      title: `= ${text}`,
      hint: 'Press Enter to copy the answer',
      icon: 'sparkle',
      run: async () => {
        try {
          await navigator.clipboard.writeText(text);
        } catch {
          /* copying is a courtesy */
        }
        close();
      },
    };
    return [row, ...special, ...withFiles];
  });

  $effect(() => {
    // A new search starts at the top.
    void query;
    active = 0;
    note = '';
  });

  onMount(() => {
    void launcherIpc.entries().then((e) => (entries = e)).catch(() => {});
    // Read when it opens, so an action added a moment ago is there.
    void actionsIpc.list().then((l) => (loadedActions = l)).catch(() => {});
    void snippetIpc.list().then((l) => (snippets = l)).catch(() => {});
    if (floating) {
      // Shown once drawn, so the window never appears empty.
      requestAnimationFrame(() => {
        void launcherIpc.ready();
        input?.focus();
      });
    } else {
      input?.focus();
    }
  });

  function onkeydown(e: KeyboardEvent) {
    if (answer) {
      if (e.key === 'Escape' || e.key === 'Enter') {
        e.preventDefault();
        close();
      }
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = Math.min(active + 1, results.length - 1);
      void scroll();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(active - 1, 0);
      void scroll();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const r = results[active];
      if (e.ctrlKey && r?.alt) void r.alt();
      else void r?.run();
    }
  }

  async function scroll() {
    await tick();
    listEl?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  }
</script>

<svelte:window onkeydown={floating ? undefined : onkeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="scrim" class:floating onclick={floating ? undefined : close}>
  <div class="box" role="dialog" aria-modal="true" aria-label="Search NomNom" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={floating ? onkeydown : undefined}>
    <div class="field">
      <Icon name="search" size={18} />
      <input
        bind:this={input}
        bind:value={query}
        placeholder={floating ? 'Search programs, files, folders and your actions' : 'Search tools and actions, or type a calculation'}
        aria-label="Search"
        autocomplete="off"
        spellcheck="false"
        role="combobox"
        aria-expanded="true"
        aria-controls="palette-list"
      />
      <kbd>Esc</kbd>
    </div>
    {#if answer}
      <div class="answer" class:bad={!answer.ok}>
        <strong>{answer.title}</strong>
        <pre translate="no">{answer.text}</pre>
        <span class="hint">Press Enter or Esc to close</span>
      </div>
    {:else}
      <ul class="list" id="palette-list" role="listbox" bind:this={listEl}>
        {#each results as r, i (r.key)}
          <li role="option" aria-selected={i === active} class:on={i === active}>
            <button tabindex="-1" onclick={() => void r.run()} onmousemove={() => (active = i)}>
              <span class="ico"><Icon name={r.icon} size={17} /></span>
              <span class="t" translate="no">{r.title}</span>
              <span class="h">{r.hint}</span>
            </button>
          </li>
        {:else}
          <li class="none">Nothing matches "{query}".</li>
        {/each}
      </ul>
      {#if note}<p class="note" role="alert">{note}</p>{/if}
    {/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: grid;
    justify-items: center;
    align-content: start;
    padding-top: 12vh;
    background: rgba(6, 5, 10, 0.56);
    animation: fade var(--dur-1) var(--ease-out);
  }
  .scrim.floating { padding: 0; background: var(--surface-1); animation: none; align-content: stretch; }
  .box {
    width: min(580px, calc(100vw - 48px));
    max-height: min(460px, 76vh);
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    background: var(--surface-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-m);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    animation: pop var(--dur-2) var(--ease-out);
  }
  .floating .box { width: 100%; max-height: none; height: 100%; border: 0; border-radius: 0; box-shadow: none; animation: none; }
  .field { display: flex; align-items: center; gap: 10px; padding: 12px 14px; border-bottom: 1px solid var(--border); color: var(--text-3); }
  input { flex: 1; min-width: 0; background: transparent; border: 0; outline: 0; font: inherit; font-size: 15px; color: var(--text); }
  input::placeholder { color: var(--text-3); }
  kbd { font: 600 11px var(--font-mono); padding: 2px 6px; border-radius: 5px; border: 1px solid var(--border-strong); color: var(--text-3); }
  .list { list-style: none; margin: 0; padding: 6px; overflow: auto; min-height: 0; }
  li button {
    width: 100%;
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-xs);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  li.on button { background: var(--surface-2); }
  .ico { display: grid; place-items: center; width: 28px; height: 28px; border-radius: var(--radius-xs); background: var(--surface-2); color: var(--accent-text); }
  li.on .ico { background: var(--surface-3); }
  .t { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .h { color: var(--text-3); font-size: 12px; white-space: nowrap; max-width: 280px; overflow: hidden; text-overflow: ellipsis; }
  .none { padding: 18px; text-align: center; color: var(--text-3); }
  .note { margin: 0; padding: 8px 16px 12px; color: var(--danger); font-size: 13px; }
  .answer { padding: 16px 18px; display: grid; gap: 8px; min-height: 0; overflow: auto; }
  .answer pre { margin: 0; white-space: pre-wrap; word-break: break-word; font: 13px/1.5 var(--font-mono); color: var(--text-2); max-height: 260px; overflow: auto; }
  .answer.bad strong { color: var(--danger); }
  .hint { color: var(--text-3); font-size: 12px; }
  @keyframes fade { from { opacity: 0; } to { opacity: 1; } }
  @keyframes pop { from { opacity: 0; transform: translateY(-6px) scale(0.98); } to { opacity: 1; transform: none; } }
</style>
