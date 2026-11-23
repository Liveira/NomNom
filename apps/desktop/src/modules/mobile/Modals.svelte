<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import Dialog from '../../lib/ui/Dialog.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import {
    mobileIpc,
    onMobileWifiPair,
    type MobileCapturedImage,
    type MobileDevice,
    type MobileSoundDisplay,
    type MobileSoundDisplayPatch,
    type MobileTimelineEntry,
    type MobileWifiDiscovery,
    type MobileWifiQrSession,
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
    onWifiConnected,
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
    onWifiConnected: (msg?: string) => void;
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

  // Wireless QR + mDNS discovery state
  let qrSession = $state<MobileWifiQrSession | null>(null);
  let qrStatus = $state('');
  let qrLoading = $state(false);
  let discovery = $state<MobileWifiDiscovery | null>(null);
  let scanning = $state(false);
  let unsubWifiPair: (() => void) | null = null;

  const usbDevices = $derived(devices.filter((d) => d.transport === 'usb' && d.state === 'device'));
  const filteredCmds = $derived(
    commands.filter((c) => {
      const q = cmdQuery.trim().toLowerCase();
      return !q || c.label.toLowerCase().includes(q) || c.hint.toLowerCase().includes(q);
    }),
  );

  async function scanWifi() {
    scanning = true;
    try {
      discovery = await mobileIpc.wifiDiscover();
    } catch {
      discovery = { services: [], saved: [] };
    } finally {
      scanning = false;
    }
  }

  async function startQr() {
    qrLoading = true;
    pairErr = '';
    qrStatus = 'Waiting for phone to scan…';
    try {
      qrSession = await mobileIpc.wifiQrStart();
    } catch (e) {
      qrSession = null;
      qrStatus = '';
      pairErr = String(e);
    } finally {
      qrLoading = false;
    }
  }

  async function stopQr() {
    qrSession = null;
    qrStatus = '';
    await mobileIpc.wifiQrStop().catch(() => {});
  }

  $effect(() => {
    if (modal === 'pair') {
      void scanWifi();
      void startQr();
      void onMobileWifiPair((ev) => {
        if (ev.status === 'error') {
          pairErr = ev.message;
          qrStatus = '';
        } else if (ev.status === 'connected') {
          pairMsg = ev.message;
          qrStatus = ev.message;
          onWifiConnected(ev.message);
          void scanWifi();
        } else {
          qrStatus = ev.message;
        }
      }).then((u) => {
        unsubWifiPair?.();
        unsubWifiPair = u;
      });
    } else {
      unsubWifiPair?.();
      unsubWifiPair = null;
      void stopQr();
    }
  });

  onDestroy(() => {
    unsubWifiPair?.();
    void mobileIpc.wifiQrStop().catch(() => {});
  });

  async function doPair() {
    pairBusy = true;
    pairErr = '';
    pairMsg = '';
    try {
      pairMsg = await onPairWifi(pairAddr, pairCode);
      pairCode = '';
      void scanWifi();
    } catch (e) {
      pairErr = String(e);
    } finally {
      pairBusy = false;
    }
  }

  async function doConnect(target = connAddr) {
    pairBusy = true;
    pairErr = '';
    pairMsg = '';
    try {
      pairMsg = await onConnectWifi(target);
      onWifiConnected(pairMsg);
      void scanWifi();
    } catch (e) {
      pairErr = String(e);
    } finally {
      pairBusy = false;
    }
  }

  async function doAutoconnect() {
    pairBusy = true;
    pairErr = '';
    pairMsg = '';
    try {
      const list = await mobileIpc.wifiAutoconnect();
      if (list.length > 0) {
        pairMsg = `Connected ${list.length} phone${list.length === 1 ? '' : 's'} over Wi-Fi (${list.join(', ')})`;
        onWifiConnected(pairMsg);
      } else {
        pairMsg = 'No nearby paired phones found yet. Enable Wireless debugging on your phone or scan the QR code.';
      }
      void scanWifi();
    } catch (e) {
      pairErr = String(e);
    } finally {
      pairBusy = false;
    }
  }

  async function doForget(endpoint: string) {
    await mobileIpc.wifiForget(endpoint).catch(() => {});
    void scanWifi();
  }

  async function doSwitchWifi(serial: string) {
    pairBusy = true;
    pairErr = '';
    pairMsg = '';
    try {
      const ep = await onEnableWifi(serial);
      pairMsg = `Connected at ${ep}. You can unplug the USB cable now.`;
      onWifiConnected(pairMsg);
      void scanWifi();
    } catch (e) {
      pairErr = String(e);
    } finally {
      pairBusy = false;
    }
  }
</script>

{#if modal === 'pair'}
  <Dialog title="Connect or pair over Wi-Fi" onclose={onClose}>
    <div class="stack">
      <!-- 1. Instant QR Code Pairing (Android 11+, zero typing) -->
      <div class="box">
        <div class="box-head">
          <h4>Scan QR code (Android 11+, no cable)</h4>
          <button class="plain sm" disabled={qrLoading} onclick={() => void startQr()}>
            <Icon name="refresh" size={13} /> New QR
          </button>
        </div>
        <div class="qr-row">
          {#if qrSession?.qr_svg}
            <div class="qr-frame" aria-label="Android wireless debugging pairing QR code">
              {@html qrSession.qr_svg}
            </div>
          {:else}
            <div class="qr-frame qr-empty">
              <span>{qrLoading ? 'Generating…' : 'QR unavailable'}</span>
            </div>
          {/if}
          <div class="qr-copy">
            <p class="hint">
              On your phone open <strong>Settings → Developer options → Wireless debugging → Pair device with QR code</strong> and scan this code.
            </p>
            <p class="hint">
              NomNom pairs and connects automatically over Wi-Fi — no IP or port typing needed. Webcam Studio, Display and Gaming all run wirelessly.
            </p>
            {#if qrStatus}
              <p class="qr-status" role="status">{qrStatus}</p>
            {/if}
          </div>
        </div>
      </div>

      <!-- 2. Nearby mDNS & Saved Wi-Fi devices -->
      <div class="box">
        <div class="box-head">
          <h4>Nearby &amp; saved Wi-Fi phones</h4>
          <div class="head-acts">
            <button class="plain sm" disabled={scanning} onclick={() => void scanWifi()}>
              <Icon name="refresh" size={13} /> {scanning ? 'Scanning…' : 'Scan'}
            </button>
            <button class="primary sm" disabled={pairBusy} onclick={() => void doAutoconnect()}>
              <Icon name="wifi" size={13} /> Auto-connect
            </button>
          </div>
        </div>

        {#if (discovery?.services.length ?? 0) > 0}
          <div class="sub-list">
            {#each discovery?.services ?? [] as svc (svc.kind + svc.endpoint)}
              <div class="usb-row">
                <div class="dev-meta">
                  <strong translate="no">{svc.endpoint}</strong>
                  <span class="hint" translate="no">
                    {svc.kind === 'connect' ? 'Ready to connect (mDNS)' : 'Waiting for pairing code'} · {svc.name}
                  </span>
                </div>
                {#if svc.kind === 'connect'}
                  <button class="primary sm" disabled={pairBusy} onclick={() => void doConnect(svc.endpoint)}>
                    Connect
                  </button>
                {:else}
                  <button class="plain sm" onclick={() => (pairAddr = svc.endpoint)}>
                    Use IP:Port
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <p class="hint">No phones advertising Wireless debugging on this Wi-Fi right now.</p>
        {/if}

        {#if (discovery?.saved.length ?? 0) > 0}
          <hr />
          <div class="sub-list">
            {#each discovery?.saved ?? [] as s (s.endpoint)}
              <div class="usb-row">
                <div class="dev-meta">
                  <strong translate="no">{s.name}</strong>
                  <span class="hint" translate="no">Saved · {s.endpoint}</span>
                </div>
                <div class="head-acts">
                  <button class="plain sm" disabled={pairBusy} onclick={() => void doConnect(s.endpoint)}>
                    Reconnect
                  </button>
                  <button class="plain sm" title="Forget saved phone" onclick={() => void doForget(s.endpoint)}>
                    <Icon name="x" size={12} />
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- 3. Switch USB to Wi-Fi -->
      {#if usbDevices.length > 0}
        <div class="box">
          <h4>Switch USB phone to Wi-Fi</h4>
          <p class="hint">Enable wireless ADB on a phone connected by cable, then unplug USB.</p>
          {#each usbDevices as d (d.serial)}
            <div class="usb-row">
              <span translate="no"><strong>{d.model}</strong> ({d.serial})</span>
              <button class="primary sm" disabled={pairBusy} onclick={() => doSwitchWifi(d.serial)}>
                <Icon name="wifi" size={13} /> Go Wireless
              </button>
            </div>
          {/each}
        </div>
      {/if}

      <!-- 4. Manual 6-digit code & IP:Port fallback -->
      <details class="manual-box">
        <summary>Manual pairing code or IP:Port</summary>
        <div class="stack" style="margin-top: 10px">
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
          <button class="plain" disabled={pairBusy || !pairAddr.trim() || !pairCode.trim()} onclick={doPair}>
            Pair with 6-digit code
          </button>

          <hr />

          <div class="row-inline">
            <input type="text" placeholder="Connect IP:Port (e.g. 192.168.1.45:41235)" bind:value={connAddr} />
            <button class="plain" disabled={pairBusy || !connAddr.trim()} onclick={() => void doConnect(connAddr)}>Connect</button>
          </div>
        </div>
      </details>

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
  .plain, .primary { display: inline-flex; align-items: center; gap: 5px; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; font-weight: 600; font-size: 13px; cursor: pointer; }
  .plain.sm, .primary.sm { padding: 5px 10px; font-size: 12px; }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .primary:hover:not(:disabled) { background: var(--accent-hover); }
  .box-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
  .head-acts { display: flex; align-items: center; gap: 6px; }
  .qr-row { display: flex; gap: 14px; align-items: center; flex-wrap: wrap; }
  .qr-frame { width: 152px; height: 152px; flex: none; border-radius: 10px; overflow: hidden; background: #fff; padding: 4px; border: 1px solid var(--border); }
  .qr-frame :global(svg) { width: 100%; height: 100%; display: block; }
  .qr-empty { display: grid; place-items: center; background: var(--surface-1); color: var(--text-3); font-size: 12px; }
  .qr-copy { flex: 1; min-width: 190px; display: grid; gap: 6px; }
  .qr-status { margin: 2px 0 0; font-size: 12.5px; font-weight: 600; color: var(--accent-text); }
  .sub-list { display: grid; gap: 8px; }
  .dev-meta { display: grid; gap: 1px; min-width: 0; }
  .manual-box { padding: 10px 14px; border-radius: var(--radius-m); background: var(--surface-2); border: 1px solid var(--border); }
  .manual-box summary { cursor: pointer; font-size: 13px; font-weight: 600; color: var(--text-2); }
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
