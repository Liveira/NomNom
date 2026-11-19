<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { miniIpc, shortcutsIpc, type ShortcutInfo } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';
  import Switch from '../../lib/ui/Switch.svelte';

  let keys = $state<Record<string, ShortcutInfo>>({});
  onMount(async () => {
    keys = Object.fromEntries((await shortcutsIpc.status()).map((s) => [s.id, s]));
  });
  const key = (id: ShortcutInfo['id']) => keys[id]?.accelerator ?? '';
  const problem = (id: ShortcutInfo['id']) => keys[id]?.error ?? null;
</script>

<header class="head">
  <h1><Icon name="wand" size={24} /> Mini tools</h1>
  <p class="sub">Small things you do all day, each on a shortcut. You can change the keys in Settings, under Shortcuts.</p>
</header>

<ul class="cards">
  {#if !isLinux}
    <li>
      <span class="ico"><Icon name="pin" size={20} /></span>
      <div class="body">
        <h2>Keep a window on top</h2>
        <p>Keeps the window you are using in front of the others, such as a video, a note or a calculator. Use the shortcut again to let it go.</p>
        <p class="keys"><span>Shortcut</span> <kbd>{key('pin_top')}</kbd></p>
        {#if problem('pin_top')}<p class="bad" role="alert">{problem('pin_top')}</p>{/if}
      </div>
    </li>
    <li>
      <span class="ico"><Icon name="brush" size={20} /></span>
      <div class="body">
        <h2>Pick a colour from the screen</h2>
        <p>Move the pointer over the colour you want, and click. Its code, like #ffa970, goes on the clipboard.</p>
        <p class="keys"><span>Shortcut</span> <kbd>{key('pick_color')}</kbd></p>
        {#if problem('pick_color')}<p class="bad" role="alert">{problem('pick_color')}</p>{/if}
        <div class="btns"><button class="plain" onclick={() => void miniIpc.pick()}>Pick a colour now</button></div>
      </div>
    </li>
    <li>
      <span class="ico"><Icon name="file-text" size={20} /></span>
      <div class="body">
        <h2>Paste as plain text</h2>
        <p>Pastes what you copied without its fonts, colours and links, only the words.</p>
        <p class="keys"><span>Shortcut</span> <kbd>{key('paste_plain')}</kbd></p>
        {#if problem('paste_plain')}<p class="bad" role="alert">{problem('paste_plain')}</p>{/if}
      </div>
    </li>
  {/if}
  <li>
    <span class="ico"><Icon name="link" size={20} /></span>
    <div class="body">
      <h2>Clean a link</h2>
      <p>Takes the tracking off the link you copied (the utm_source and fbclid parts), so it is the page and nothing else.</p>
      <p class="keys"><span>Shortcut</span> <kbd>{key('clean_link')}</kbd></p>
      {#if problem('clean_link')}<p class="bad" role="alert">{problem('clean_link')}</p>{/if}
      <div class="btns"><button class="plain" onclick={() => void miniIpc.cleanNow()}>Clean the link on the clipboard</button></div>
      {#if !isLinux}
        <div class="sw">
          <span>Clean every link as soon as I copy it</span>
          <Switch
            checked={app.settings?.mini.clean_links_auto ?? false}
            label="Clean every link as soon as I copy it"
            onchange={(v) => app.update({ cleanLinksAuto: v })}
          />
        </div>
      {/if}
    </div>
  </li>
</ul>

<style>
  .head { display: grid; gap: 8px; margin-bottom: 18px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  h2 { font-size: 16px; margin: 0 0 4px; }
  .sub { color: var(--text-3); font-size: 13px; margin: 0; }
  .cards { list-style: none; margin: 0; padding: 0; display: grid; gap: 10px; max-width: 760px; }
  li { display: flex; gap: 14px; padding: 14px 16px; border-radius: var(--radius-s); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border); }
  .ico { flex: none; width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); }
  .body { flex: 1; min-width: 0; display: grid; gap: 6px; }
  p { margin: 0; color: var(--text-2); font-size: 13.5px; }
  kbd { font: 600 12px var(--font-mono); padding: 2px 8px; border-radius: 6px; border: 1px solid var(--border-strong); color: var(--text); }
  .keys { display: flex; align-items: center; gap: 8px; color: var(--text-3); font-size: 12.5px; }
  .bad { color: var(--danger); }
  .btns { display: flex; gap: 8px; }
  .sw { display: flex; align-items: center; justify-content: space-between; gap: 12px; font-weight: 600; font-size: 13px; padding-top: 4px; }
  button.plain { display: inline-flex; align-items: center; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
