<script lang="ts">
  import { app } from '../../app/state.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { ipc, type SettingsPatch, type Theme } from '../../lib/ipc';
  import { downloads } from '../downloads/store.svelte';

  const themes: { value: Theme; label: string }[] = [
    { value: 'system', label: 'System' },
    { value: 'dark', label: 'Dark' },
    { value: 'light', label: 'Light' },
  ];
  const theme = $derived(app.settings?.appearance.theme ?? 'system');
  const closeToTray = $derived(app.settings?.general.close_to_tray ?? true);
  const dl = $derived(app.settings?.downloads);

  let defaultDir = $state('');
  let message = $state('');
  let updating = $state(false);
  void ipc.defaultDownloadDir().then((d) => (defaultDir = d));
  void downloads.refreshTools();

  async function chooseFolder() {
    const picked = await open({ directory: true, multiple: false, defaultPath: dl?.output_dir ?? defaultDir });
    if (typeof picked === 'string') await save({ outputDir: picked });
  }

  async function save(patch: SettingsPatch) {
    message = '';
    try {
      await app.update(patch);
    } catch (e) {
      message = String(e);
    }
  }

  async function updateYtdlp() {
    updating = true;
    message = '';
    try {
      const v = await ipc.updateYtdlp();
      message = v ? `yt-dlp is up to date (${v}).` : 'Checked for updates.';
    } catch (e) {
      message = String(e);
    } finally {
      updating = false;
      await downloads.refreshTools();
    }
  }
</script>

<h1>Settings</h1>

<section class="group">
  <h3>Appearance</h3>
  <div class="row">
    <div>
      <div class="label">Theme</div>
    </div>
    <div class="segmented" role="radiogroup" aria-label="Theme">
      {#each themes as t (t.value)}
        <button
          role="radio"
          aria-checked={theme === t.value}
          class:on={theme === t.value}
          onclick={() => app.setTheme(t.value)}>{t.label}</button
        >
      {/each}
    </div>
  </div>
</section>

<section class="group">
  <h3>General</h3>
  <div class="row">
    <div>
      <div class="label">Keep running in the tray when the window is closed</div>
      <div class="hint">Downloads and shortcuts keep working. Turn this off to quit instead.</div>
    </div>
    <button
      class="switch"
      role="switch"
      aria-checked={closeToTray}
      aria-label="Keep running in the tray when the window is closed"
      class:on={closeToTray}
      onclick={() => app.setCloseToTray(!closeToTray)}><span></span></button
    >
  </div>
</section>

<section class="group">
  <h3>Downloads</h3>
  <div class="row">
    <div>
      <div class="label">Save to</div>
      <div class="hint path">{dl?.output_dir ?? defaultDir}</div>
    </div>
    <div class="buttons">
      <button class="plain" onclick={chooseFolder}>Choose folder</button>
      {#if dl?.output_dir}<button class="plain" onclick={() => save({ outputDir: '' })}>Use Downloads</button>{/if}
    </div>
  </div>
  <div class="row">
    <div class="label">Downloads at the same time</div>
    <select
      aria-label="Downloads at the same time"
      value={dl?.max_concurrent ?? 3}
      onchange={(e) => save({ maxConcurrent: Number(e.currentTarget.value) })}
    >
      {#each [1, 2, 3, 4, 5, 6, 8] as n (n)}<option value={n}>{n}</option>{/each}
    </select>
  </div>
  <div class="row">
    <div>
      <div class="label">Allow downloading the same link again</div>
      <div class="hint">When off, NOMNOM asks before downloading something you already have.</div>
    </div>
    <button class="switch" role="switch" aria-label="Allow downloading the same link again" aria-checked={dl?.allow_duplicates ?? false} class:on={dl?.allow_duplicates} onclick={() => save({ allowDuplicates: !dl?.allow_duplicates })}><span></span></button>
  </div>
  <div class="row">
    <div class="label">Show a notification when downloads finish</div>
    <button class="switch" role="switch" aria-label="Show a notification when downloads finish" aria-checked={dl?.notify ?? true} class:on={dl?.notify} onclick={() => save({ notifyDownloads: !dl?.notify })}><span></span></button>
  </div>
  <div class="row">
    <div>
      <div class="label">Use Firefox cookies for sites that need a sign-in</div>
      <div class="hint">NOMNOM reads your Firefox cookies only while downloading, and only on this computer. Off by default.</div>
    </div>
    <button class="switch" role="switch" aria-label="Use Firefox cookies" aria-checked={dl?.use_firefox_cookies ?? false} class:on={dl?.use_firefox_cookies} onclick={() => save({ firefoxCookies: !dl?.use_firefox_cookies })}><span></span></button>
  </div>

  <h3 class="sub">Tools</h3>
  {#if downloads.tools}
    {#each [['yt-dlp', downloads.tools.ytdlp, 'ytdlp'], ['FFmpeg', downloads.tools.ffmpeg, 'ffmpeg']] as const as [name, tool, id] (id)}
      <div class="row">
        <div>
          <div class="label">{name}</div>
          <div class="hint">
            {#if tool.path}{tool.version ?? 'Installed'}{tool.managed ? '' : ' · found on this computer'}{:else}Not installed{/if}
          </div>
        </div>
        <div class="buttons">
          {#if !tool.path}
            <button class="plain" disabled={downloads.installing != null} onclick={() => downloads.install(id)}>Get {name}</button>
          {:else if id === 'ytdlp' && tool.managed}
            <button class="plain" disabled={updating} onclick={updateYtdlp}>{updating ? 'Checking…' : 'Update'}</button>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
  {#if message}<p class="hint" role="status">{message}</p>{/if}
</section>

<style>
  .path { font-variant-numeric: tabular-nums; word-break: break-all; }
  .buttons { display: flex; gap: 8px; flex: none; }
  .plain, select {
    padding: 6px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); cursor: pointer;
  }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .plain:disabled { opacity: 0.5; cursor: default; }
  .group .row + .row { margin-top: 14px; }
  h3.sub { margin-top: 22px; }
  h1 { margin-bottom: 20px; }
  .group {
    max-width: 640px;
    margin-bottom: 16px;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    padding: 16px 18px;
  }
  h3 { margin-bottom: 12px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }

  .segmented { display: inline-flex; background: var(--surface-2); border-radius: var(--radius-s); padding: 2px; }
  .segmented button {
    border: 0; background: transparent; color: var(--text-2); padding: 5px 12px;
    border-radius: 5px; cursor: pointer; transition: background var(--dur-1), color var(--dur-1);
  }
  .segmented button.on { background: var(--surface-3); color: var(--text); }

  .switch {
    flex: none; width: 38px; height: 22px; border-radius: 11px; border: 0; padding: 2px;
    background: var(--surface-3); cursor: pointer; transition: background var(--dur-2);
  }
  .switch span {
    display: block; width: 18px; height: 18px; border-radius: 50%; background: var(--text-2);
    transition: transform var(--dur-2) var(--ease-spring), background var(--dur-2);
  }
  .switch.on { background: var(--accent); }
  .switch.on span { transform: translateX(16px); background: var(--on-accent); }
</style>
