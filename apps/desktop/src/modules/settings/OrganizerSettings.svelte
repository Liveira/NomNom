<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { app } from '../../app/state.svelte';
  import { orgIpc, type SettingsPatch } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';

  const s = $derived(app.settings?.organizer);
  let defaultDest = $state('');
  let message = $state('');
  void orgIpc.defaults().then((d) => (defaultDest = d.dest_root));

  async function save(patch: SettingsPatch) {
    message = '';
    try {
      await app.update(patch);
      defaultDest = (await orgIpc.defaults()).dest_root;
    } catch (e) {
      message = String(e);
    }
  }

  async function choose() {
    const picked = await open({ directory: true, multiple: false, defaultPath: s?.dest_root ?? defaultDest });
    if (typeof picked === 'string') await save({ organizerDest: picked });
  }

  const dest = $derived(s?.dest_root ?? defaultDest);
</script>

<section class="group" aria-labelledby="org">
  <h3 id="org">File Organizer</h3>

  <div class="row">
    <div>
      <div class="label">Organized files go to</div>
      <div class="hint path">{dest}</div>
      <div class="hint">Inside it: Videos, Audio, Images and Documents.</div>
    </div>
    <div class="buttons">
      <button class="plain" onclick={choose}>Choose folder</button>
      {#if s?.dest_root}<button class="plain" onclick={() => save({ organizerDest: '' })}>Use Documents</button>{/if}
    </div>
  </div>

  <div class="row">
    <div>
      <div class="label">Save NomNom Downloader's files here too</div>
      <div class="hint">New downloads go straight into those folders, already sorted. Turn this off to use the folder in the downloader's settings instead.</div>
    </div>
    <Switch checked={s?.link_downloads ?? true} label="Save NomNom Downloader's files in the organizer folders" onchange={(v) => save({ linkDownloads: v })} />
  </div>

  <div class="row">
    <div>
      <div class="label">Organize new downloads automatically</div>
      <div class="hint">
        Watches your Downloads folder and moves a new file only after it has finished downloading and stopped changing.
        It uses the same safety checks as a manual scan, and every move can be undone. Off by default.
      </div>
    </div>
    <Switch checked={s?.auto_enabled ?? false} label="Organize new downloads automatically" onchange={(v) => save({ autoOrganize: v })} />
  </div>

  <div class="row">
    <div>
      <div class="label">When a file with the same name is already there</div>
      <div class="hint">Nothing is ever replaced.</div>
    </div>
    <div class="segmented" role="radiogroup" aria-label="When a file with the same name is already there">
      <button role="radio" aria-checked={s?.conflict !== 'keep_both'} class:on={s?.conflict !== 'keep_both'} onclick={() => save({ organizerConflict: 'skip' })}>Leave it</button>
      <button role="radio" aria-checked={s?.conflict === 'keep_both'} class:on={s?.conflict === 'keep_both'} onclick={() => save({ organizerConflict: 'keep_both' })}>Keep both</button>
    </div>
  </div>

  {#if message}<p class="hint bad" role="alert">{message}</p>{/if}
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .row + .row { margin-top: 14px; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .bad { color: var(--danger); }
  .path { font-family: Consolas, monospace; word-break: break-all; user-select: text; }
  .buttons { display: flex; gap: 8px; flex: none; }
  .plain { padding: 6px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); cursor: pointer; }
  .plain:hover { background: var(--surface-3); }
  .segmented { display: inline-flex; background: var(--surface-2); border-radius: var(--radius-s); padding: 2px; flex: none; }
  .segmented button { border: 0; background: transparent; color: var(--text-2); padding: 5px 12px; border-radius: 6px; cursor: pointer; }
  .segmented button.on { background: var(--surface-3); color: var(--text); }
</style>
