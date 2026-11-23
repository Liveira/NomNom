<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import Dialog from '../../lib/ui/Dialog.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import type {
    MobileCapturedImage,
    MobileDevice,
    MobileSoundDisplay,
    MobileSoundDisplayPatch,
    MobileTimelineEntry,
  } from '../../lib/ipc';
  import { relativeSecs, timelineIcon } from './logic';

  let {
    modal,
    devices,
    soundDisplay,
    timeline,
    captured,
    commands,
    onClose,
    onPairWifi,
    onConnectWifi,
    onEnableWifi,
    onApplySoundDisplay,
    onClearTimeline,
    onRunCommand,
    onOpenPath,
  }: {
    modal: 'none' | 'pair' | 'sound' | 'timeline' | 'commands' | 'capture';
    devices: MobileDevice[];
    soundDisplay: MobileSoundDisplay | null;
    timeline: MobileTimelineEntry[];
    captured: MobileCapturedImage | null;
    commands: { id: string; label: string; hint: string; icon: string }[];
    onClose: () => void;
    onPairWifi: (addr: string, code: string) => Promise<string>;
    onConnectWifi: (addr: string) => Promise<string>;
    onEnableWifi: (serial: string) => Promise<string>;
    onApplySoundDisplay: (patch: MobileSoundDisplayPatch) => Promise<void>;
    onClearTimeline: () => void;
    onRunCommand: (id: string) => void;
    onOpenPath: (path: string) => void;
  } = $props();

  let pairAddr = $state('');
  let pairCode = $state('');
  let connAddr = $state('');
  let pairMsg = $state('');
  let pairErr = $state('');
  let pairBusy = $state(false);
  let cmdQuery = $state('');

  const usbDevices = $derived(devices.filter((d) => d.transport === 'usb' && d.state === 'device'));
  const filteredCmds = $derived(
    commands.filter((c) => {
      const q = cmdQuery.trim().toLowerCase();
      return !q || c.label.toLowerCase().includes(q) || c.hint.toLowerCase().includes(q);
    }),
  );

  async function doPair() {
    pairBusy = true;
    pairErr = '';
    pairMsg = '';
    try {
      pairMsg = await onPairWifi(pairAddr, pairCode);
      pairCode = '';
    } catch (e) {
      pairErr = String(e);
    } finally {
      pairBusy = false;
    }
  }

  async function doConnect() {
    pairBusy = true;
    pairErr = '';
    pairMsg = '';
    try {
      pairMsg = await onConnectWifi(connAddr);
    } catch (e) {
      pairErr = String(e);
    } finally {
      pairBusy = false;
    }
  }

  async function doSwitchWifi(serial: string) {
    pairBusy = true;
    pairErr = '';
    pairMsg = '';
    try {
      const ep = await onEnableWifi(serial);
      pairMsg = `Connected at ${ep}. You can unplug the USB cable now.`;
    } catch (e) {
      pairErr = String(e);
    } finally {
      pairBusy = false;
    }
  }
</script>

{#if modal === 'pair'}
  <Dialog title="Connect or pair an Android phone" onclose={onClose}>
    <div class="stack">
      <div class="box">
        <h4>Pair over Wi-Fi (Android 11+)</h4>
        <p class="hint">
          On your phone open Settings → Developer options → Wireless debugging → Pair device with pairing code.
        </p>
        <div class="row2">
          <label>
            <span>Pairing IP:Port</span>
            <input type="text" placeholder="192.168.1.45:37123" bind:value={pairAddr} />
          </label>
          <label>
            <span>6-digit code</span>
            <input type="text" placeholder="482910" bind:value={pairCode} />
          </label>
        </div>
        <button class="primary" disabled={pairBusy || !pairAddr.trim() || !pairCode.trim()} onclick={doPair}>
          Pair device
        </button>
      </div>

      <div class="box">
        <h4>Connect to an already paired phone</h4>
        <p class="hint">Enter the main IP address & Port shown at the top of Wireless debugging.</p>
        <div class="row-inline">
          <input type="text" placeholder="192.168.1.45:41235" bind:value={connAddr} />
          <button class="plain" disabled={pairBusy || !connAddr.trim()} onclick={doConnect}>Connect</button>
        </div>
      </div>

      {#if usbDevices.length > 0}
        <div class="box">
          <h4>Switch USB phone to Wi-Fi</h4>
          <p class="hint">Enable wireless ADB on a phone connected by cable, then unplug USB.</p>
          {#each usbDevices as d (d.serial)}
            <div class="usb-row">
              <span translate="no"><strong>{d.model}</strong> ({d.serial})</span>
              <button class="plain" disabled={pairBusy} onclick={() => doSwitchWifi(d.serial)}>
                Switch to Wi-Fi
              </button>
            </div>
          {/each}
        </div>
      {/if}

      {#if pairMsg}<p class="ok" role="status">{pairMsg}</p>{/if}
      {#if pairErr}<p class="err" role="alert">{pairErr}</p>{/if}
    </div>
    {#snippet actions()}
      <button class="plain" onclick={onClose}>Close</button>
    {/snippet}
  </Dialog>
{:else if modal === 'sound' && soundDisplay}
  <Dialog title="Sound & display" onclose={onClose}>
    <div class="stack">
      <div class="slider-row">
        <div class="sl-head"><span>Media volume</span><strong class="tabular">{soundDisplay.media_vol} / {soundDisplay.media_max}</strong></div>
        <input
          type="range"
          min="0"
          max={soundDisplay.media_max}
          value={soundDisplay.media_vol}
          onchange={(e) => void onApplySoundDisplay({ mediaVol: Number(e.currentTarget.value) })}
        />
      </div>
      <div class="slider-row">
        <div class="sl-head"><span>Ring volume</span><strong class="tabular">{soundDisplay.ring_vol} / {soundDisplay.ring_max}</strong></div>
        <input
          type="range"
          min="0"
          max={soundDisplay.ring_max}
          value={soundDisplay.ring_vol}
          onchange={(e) => void onApplySoundDisplay({ ringVol: Number(e.currentTarget.value) })}
        />
      </div>
      <div class="slider-row">
        <div class="sl-head"><span>Alarm volume</span><strong class="tabular">{soundDisplay.alarm_vol} / {soundDisplay.alarm_max}</strong></div>
        <input
          type="range"
          min="0"
          max={soundDisplay.alarm_max}
          value={soundDisplay.alarm_vol}
          onchange={(e) => void onApplySoundDisplay({ alarmVol: Number(e.currentTarget.value) })}
        />
      </div>
      <div class="slider-row">
        <div class="sl-head"><span>Notification volume</span><strong class="tabular">{soundDisplay.notif_vol} / {soundDisplay.notif_max}</strong></div>
        <input
          type="range"
          min="0"
          max={soundDisplay.notif_max}
          value={soundDisplay.notif_vol}
          onchange={(e) => void onApplySoundDisplay({ notifVol: Number(e.currentTarget.value) })}
        />
      </div>

      <hr />

      <div class="slider-row">
        <div class="sl-head"><span>Screen brightness</span><strong class="tabular">{Math.round((soundDisplay.brightness / 255) * 100)}%</strong></div>
        <input
          type="range"
          min="5"
          max="255"
          value={soundDisplay.brightness}
          onchange={(e) => void onApplySoundDisplay({ brightness: Number(e.currentTarget.value) })}
        />
      </div>
      <div class="toggle-line">
        <span>Adaptive brightness</span>
        <Switch
          checked={soundDisplay.auto_brightness}
          label="Adaptive brightness"
          onchange={(v) => void onApplySoundDisplay({ autoBrightness: v })}
        />
      </div>
      <div class="toggle-line">
        <span>Screen timeout</span>
        <select
          value={soundDisplay.screen_timeout_ms}
          onchange={(e) => void onApplySoundDisplay({ screenTimeoutMs: Number(e.currentTarget.value) })}
        >
          <option value={30000}>30 seconds</option>
          <option value={60000}>1 minute</option>
          <option value={120000}>2 minutes</option>
          <option value={300000}>5 minutes</option>
          <option value={600000}>10 minutes</option>
        </select>
      </div>
    </div>
    {#snippet actions()}
      <button class="primary" onclick={onClose}>Done</button>
    {/snippet}
  </Dialog>
{:else if modal === 'timeline'}
  <Dialog title="Timeline" onclose={onClose}>
    {#if timeline.length === 0}
      <p class="hint">No activity recorded yet.</p>
    {:else}
      <ul class="tl-full">
        {#each timeline as item (item.id)}
          <li>
            <span class="tl-ico"><Icon name={timelineIcon(item.kind)} size={15} /></span>
            <div class="tl-copy">
              <div class="tl-t" translate="no">{item.title}</div>
              <div class="tl-d" translate="no">{item.detail}</div>
            </div>
            <span class="tl-when tabular">{relativeSecs(item.at)}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#snippet actions()}
      {#if timeline.length > 0}
        <button class="plain" onclick={onClearTimeline}>Clear history</button>
      {/if}
      <button class="primary" onclick={onClose}>Close</button>
    {/snippet}
  </Dialog>
{:else if modal === 'commands'}
  <Dialog title="Commands" onclose={onClose}>
    <div class="stack">
      <input
        type="search"
        placeholder="Filter phone commands…"
        bind:value={cmdQuery}
        aria-label="Filter phone commands"
      />
      <ul class="cmd-list">
        {#each filteredCmds as c (c.id)}
          <li>
            <button
              class="cmd-row"
              onclick={() => {
                onClose();
                onRunCommand(c.id);
              }}
            >
              <span class="tl-ico"><Icon name={c.icon} size={15} /></span>
              <span class="tl-copy">
                <strong>{c.label}</strong>
                <span class="hint">{c.hint}</span>
              </span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  </Dialog>
{:else if modal === 'capture' && captured}
  <Dialog title={captured.name} onclose={onClose}>
    <div class="stack">
      <img class="cap-img" src={captured.data_url} alt={captured.name} />
      <p class="hint" translate="no">Saved to {captured.path}</p>
    </div>
    {#snippet actions()}
      <button class="plain" onclick={() => onOpenPath(captured.path)}>Open file</button>
      <button class="primary" onclick={onClose}>Done</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .stack { display: grid; gap: 12px; }
  .box { display: grid; gap: 8px; padding: 12px 14px; border-radius: var(--radius-m); background: var(--surface-2); border: 1px solid var(--border); }
  h4 { margin: 0; font-size: 13.5px; }
  .hint { margin: 0; font-size: 12.5px; color: var(--text-3); line-height: 1.45; }
  .row2 { display: grid; grid-template-columns: 1.3fr 1fr; gap: 8px; }
  .row-inline { display: flex; gap: 8px; }
  .row-inline input { flex: 1; }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--text-3); font-weight: 600; }
  input[type='text'], input[type='search'], select {
    padding: 7px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-1); color: var(--text); font-size: 13px;
  }
  input[type='range'] { width: 100%; accent-color: var(--accent); }
  .usb-row, .toggle-line { display: flex; align-items: center; justify-content: space-between; gap: 10px; font-size: 13px; }
  .slider-row { display: grid; gap: 4px; }
  .sl-head { display: flex; justify-content: space-between; font-size: 13px; }
  hr { border: 0; border-top: 1px solid var(--border); margin: 4px 0; }
  .ok { color: var(--ok); margin: 0; font-size: 13px; }
  .err { color: var(--danger); margin: 0; font-size: 13px; }
  .plain, .primary { padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; font-weight: 600; font-size: 13px; cursor: pointer; }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .primary:hover:not(:disabled) { background: var(--accent-hover); }
  button:disabled { opacity: 0.45; cursor: default; }
  .tl-full, .cmd-list { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; max-height: 360px; overflow: auto; }
  .tl-full li { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); }
  .tl-ico { width: 30px; height: 30px; flex: none; border-radius: 8px; background: var(--surface-1); border: 1px solid var(--border); color: var(--accent-text); display: grid; place-items: center; }
  .tl-copy { flex: 1; min-width: 0; display: grid; gap: 1px; }
  .tl-t { font-size: 13px; font-weight: 600; }
  .tl-d { font-size: 12px; color: var(--text-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tl-when { font-size: 11.5px; color: var(--text-3); flex: none; }
  .cmd-row { width: 100%; display: flex; align-items: center; gap: 10px; padding: 8px 10px; border: 1px solid var(--border); border-radius: var(--radius-s); background: var(--surface-2); text-align: left; cursor: pointer; }
  .cmd-row:hover { background: var(--surface-3); border-color: var(--border-strong); }
  .cap-img { max-width: 100%; max-height: 400px; object-fit: contain; border-radius: var(--radius-s); background: var(--surface-2); }
</style>
