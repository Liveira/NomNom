<script lang="ts">
  import { onDestroy } from 'svelte';
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { gameModeIpc } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';
  import { every } from '../../lib/poll';
  import Switch from '../../lib/ui/Switch.svelte';

  const cfg = $derived(app.settings?.game);
  let on = $state(false);
  let game = $state<string | null>(null);
  let programs = $state((app.settings?.game.close ?? []).join(', '));
  let saved = $state('');

  async function look() {
    const s = await gameModeIpc.status().catch(() => null);
    if (s) {
      on = s.on;
      game = s.game;
    }
  }
  void look();
  const timer = every(3000, () => void look());
  onDestroy(timer);

  async function saveList() {
    await app.update({ gameClose: programs });
    programs = (app.settings?.game.close ?? []).join(', ');
    saved = 'Saved.';
    setTimeout(() => (saved = ''), 1800);
  }
</script>

<header class="head">
  <h1><Icon name="playtime" size={24} /> Game mode</h1>
  <p class="sub">When a game has been in front for a few seconds, NomNom can keep notices from covering it, ask Windows for its fastest power plan, and close the background programs you pick. A few seconds after the game is gone, the power plan is put back. Playtime is what tells a game from other programs.</p>
</header>

{#if isLinux}
  <p class="hint">Game mode works on Windows for now.</p>
{:else if cfg}
  <section class="card">
    <div class="row main">
      <div>
        <div class="label">Use game mode</div>
        <div class="hint">{#if on}On right now{#if game}: <span translate="no">{game}</span>{/if}.{:else}Waiting for a game.{/if}</div>
      </div>
      <Switch checked={cfg.enabled} label="Use game mode" onchange={(v) => app.update({ gameEnabled: v })} />
    </div>
    <div class="row" class:dim={!cfg.enabled}>
      <div>
        <div class="label">Hold back notices</div>
        <div class="hint">Errors, warnings and reminders still show. The rest wait in Notifications.</div>
      </div>
      <Switch checked={cfg.silence} label="Hold back notices" onchange={(v) => app.update({ gameSilence: v })} />
    </div>
    <div class="row" class:dim={!cfg.enabled}>
      <div>
        <div class="label">Use the fastest power plan</div>
        <div class="hint">Uses more power while you play, and the plan you had comes back after. If NomNom closes in the middle of a game, it puts it back the next time it starts.</div>
      </div>
      <Switch checked={cfg.power_plan} label="Use the fastest power plan" onchange={(v) => app.update({ gamePowerPlan: v })} />
    </div>
    <div class="col" class:dim={!cfg.enabled}>
      <div class="label">Close these programs when a game starts</div>
      <div class="hint">File names such as OneDrive.exe, separated by commas. NomNom asks each one to close, like its close button does, so a program with unsaved work can ask you first. It does not start them again. Windows itself and NomNom can't be on the list.</div>
      <textarea bind:value={programs} rows="2" spellcheck="false" translate="no" placeholder="OneDrive.exe, Teams.exe" aria-label="Programs to close"></textarea>
      <div class="row-end">
        <span class="ok" role="status">{saved}</span>
        <button class="plain" onclick={() => void saveList()}>Save the list</button>
      </div>
    </div>
  </section>
{/if}

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 2px 0 0; }
  .card { max-width: 700px; padding: 8px 18px 14px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border-strong); }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 12px 0; }
  .row + .row, .col { border-top: 1px solid var(--border); }
  .col { display: grid; gap: 6px; padding: 12px 0 2px; }
  .main { padding-top: 14px; }
  .label { font-weight: 600; }
  .dim { opacity: 0.55; }
  textarea { font: inherit; padding: 8px 10px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); resize: vertical; width: 100%; box-sizing: border-box; font-family: var(--font-mono); font-size: 13px; }
  textarea:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .row-end { display: flex; justify-content: flex-end; align-items: center; gap: 12px; }
  .ok { color: var(--ok); font-size: 13px; }
  .plain { padding: 6px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  .plain:hover { background: var(--surface-3); }
  .plain:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
