<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { onSubscriptionsChanged, subIpc, type Subscription } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import { checkedText, EVERY_CHOICES } from './subscriptions';

  let list = $state<Subscription[]>([]);
  let adding = $state(false);
  let url = $state('');
  let kind = $state<'video' | 'audio'>('video');
  let every = $state(6);
  let backfill = $state(0);
  let busy = $state(false);
  let checking = $state<string | null>(null);
  let error = $state('');
  let note = $state('');

  async function refresh() {
    list = await subIpc.list().catch(() => []);
  }
  void refresh();
  let stop: (() => void) | null = null;
  void onSubscriptionsChanged(() => void refresh()).then((u) => (stop = u));
  onDestroy(() => stop?.());

  async function add() {
    busy = true;
    error = '';
    note = '';
    try {
      const s = await subIpc.add(url.trim(), kind, every, backfill);
      note = s.started > 0 ? `Following ${s.title}. ${s.started} ${s.started === 1 ? 'video is' : 'videos are'} downloading.` : `Following ${s.title}. New videos will download by themselves.`;
      url = '';
      adding = false;
      backfill = 0;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function checkNow(s: Subscription) {
    checking = s.id;
    error = '';
    note = '';
    try {
      const n = await subIpc.checkNow(s.id);
      note = n > 0 ? `${n} new ${n === 1 ? 'video' : 'videos'} started from ${s.title}.` : `Nothing new from ${s.title}.`;
    } catch (e) {
      error = String(e);
    } finally {
      checking = null;
      await refresh();
    }
  }
</script>

<section class="subs" aria-labelledby="subs-h">
  <div class="head">
    <h2 id="subs-h"><Icon name="bell" size={17} /> Following</h2>
    <button class="plain" onclick={() => (adding = !adding)}><Icon name="plus" size={15} /> Follow a channel or playlist</button>
  </div>
  {#if list.length === 0 && !adding}
    <p class="hint">Follow a channel or a playlist and NomNom downloads each new video by itself, with your usual format.</p>
  {/if}

  {#if adding}
    <form class="form" onsubmit={(e) => { e.preventDefault(); void add(); }}>
      <label class="wide">
        <span>Link to the channel or playlist</span>
        <input type="url" placeholder="https://www.youtube.com/@channel/videos" bind:value={url} spellcheck="false" required />
      </label>
      <label>
        <span>Get</span>
        <select bind:value={kind}>
          <option value="video">Video</option>
          <option value="audio">Audio only</option>
        </select>
      </label>
      <label>
        <span>Look for new ones</span>
        <select bind:value={every}>
          {#each EVERY_CHOICES as c (c.hours)}<option value={c.hours}>{c.label}</option>{/each}
        </select>
      </label>
      <label>
        <span>Also get the latest</span>
        <select bind:value={backfill}>
          <option value={0}>None, only new ones</option>
          <option value={1}>1 video</option>
          <option value={3}>3 videos</option>
          <option value={5}>5 videos</option>
          <option value={10}>10 videos</option>
        </select>
      </label>
      <div class="actions">
        <button type="button" class="plain" onclick={() => (adding = false)}>Cancel</button>
        <button type="submit" class="primary" disabled={busy || url.trim() === ''}>{busy ? 'Looking…' : 'Follow'}</button>
      </div>
    </form>
  {/if}

  <ul class="list">
    {#each list as s (s.id)}
      <li class:off={!s.enabled}>
        <span class="ico"><Icon name={s.kind === 'audio' ? 'music' : 'download'} size={16} /></span>
        <span class="what">
          <strong translate="no" title={s.url}>{s.title}</strong>
          <span class="when">{s.kind === 'audio' ? 'Audio' : 'Video'} · {checkedText(s.last_checked, s.every_hours)} · {s.started} {s.started === 1 ? 'video' : 'videos'} started</span>
        </span>
        <button class="plain small" disabled={checking === s.id} onclick={() => checkNow(s)}>{checking === s.id ? 'Looking…' : 'Look now'}</button>
        <Switch checked={s.enabled} label={`Follow ${s.title}`} onchange={(on) => subIpc.toggle(s.id, on)} />
        <button class="x" aria-label="Stop following" title="Stop following" onclick={() => subIpc.remove(s.id)}><Icon name="trash" size={15} /></button>
      </li>
    {/each}
  </ul>
  {#if note}<p class="ok" role="status">{note}</p>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</section>

<style>
  .subs { margin: 16px 0; padding: 14px 16px; border-radius: var(--radius-m); background: color-mix(in srgb, var(--surface-1) 90%, transparent); border: 1px solid var(--border-strong); }
  .head { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  h2 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 16px; }
  .hint { color: var(--text-3); font-size: 12.5px; margin: 8px 0 0; }
  .form { margin-top: 12px; padding: 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-end; }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--text-3); font-weight: 600; }
  .wide { flex: 1 1 100%; }
  input, select { background: var(--surface-1); color: var(--text); border: 1px solid var(--border-strong); border-radius: var(--radius-xs); padding: 8px 10px; font: inherit; }
  .actions { margin-left: auto; display: flex; gap: 8px; }
  .list { list-style: none; margin: 10px 0 0; padding: 0; display: grid; gap: 8px; }
  li { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); }
  li.off { opacity: 0.55; }
  .ico { width: 30px; height: 30px; border-radius: 9px; display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); flex: none; }
  .what { display: grid; gap: 1px; min-width: 0; flex: 1; }
  .what strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .when { color: var(--text-3); font-size: 12px; }
  button.plain, button.primary { display: inline-flex; align-items: center; gap: 6px; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; }
  button.small { padding: 4px 10px; font-size: 12.5px; }
  button.plain:hover:not(:disabled) { background: var(--surface-3); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button:disabled { opacity: 0.5; cursor: default; }
  .x { border: 0; background: none; color: var(--text-3); cursor: pointer; padding: 6px; border-radius: 8px; display: grid; }
  .x:hover { background: var(--surface-3); color: var(--text); }
  .ok { color: var(--ok); margin: 8px 0 0; font-size: 13px; }
  .err { color: var(--danger); margin: 8px 0 0; font-size: 13px; }
</style>
