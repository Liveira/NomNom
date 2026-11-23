<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import {
    mobileIpc,
    type MobileDevice,
    type MobileDisplayInfo,
    type MobileMirrorOptions,
  } from '../../lib/ipc';

  let {
    device,
    onNotice,
    onError,
  }: {
    device: MobileDevice | null;
    onNotice: (msg: string) => void;
    onError: (msg: string) => void;
  } = $props();

  const serial = $derived(device?.state === 'device' ? device.serial : null);

  let displays = $state<MobileDisplayInfo[]>([]);
  let loadingDisplays = $state(false);

  // Virtual display config
  let vdSize = $state('1920x1080');
  let vdDpi = $state(240);
  let startApp = $state('');
  let videoCodec = $state<'h264' | 'h265' | 'av1'>('h265');
  let maxFps = $state(60);
  let bitRateMbps = $state(12);
  let audio = $state(true);
  let turnScreenOff = $state(false);
  let keepAliveOnClose = $state(true);
  let noDecorations = $state(false);
  let keyboardUhid = $state(true);
  let mouseUhid = $state(false);
  let gamepadUhid = $state(false);
  let borderless = $state(false);
  let alwaysOnTop = $state(false);

  async function loadDisplays() {
    if (!serial) return;
    loadingDisplays = true;
    try {
      displays = await mobileIpc.displays(serial);
    } catch (e) {
      onError(String(e));
    } finally {
      loadingDisplays = false;
    }
  }

  $effect(() => {
    if (serial) void loadDisplays();
  });

  function pickPreset(size: string, dpi: number) {
    vdSize = size;
    vdDpi = dpi;
  }

  async function launchVirtualDisplay() {
    if (!serial) return;
    try {
      const opts: MobileMirrorOptions = {
        newDisplay: true,
        displaySize: vdSize,
        displayDpi: vdDpi,
        startApp: startApp.trim() || undefined,
        videoCodec,
        maxFps,
        bitRateMbps,
        audio,
        turnScreenOff,
        stayAwake: true,
        keepAliveOnClose,
        noDecorations,
        keyboardUhid,
        mouseUhid,
        gamepadUhid,
        borderless,
        alwaysOnTop,
      };
      await mobileIpc.mirror(serial, opts);
      onNotice(`Launched virtual display (${vdSize} @ ${vdDpi} dpi)`);
    } catch (e) {
      onError(String(e));
    }
  }

  async function launchGamingMirror() {
    if (!serial) return;
    try {
      await mobileIpc.mirror(serial, {
        maxSize: 0,
        videoCodec,
        maxFps: Math.max(60, maxFps),
        bitRateMbps: Math.max(16, bitRateMbps),
        audio: true,
        stayAwake: true,
        turnScreenOff,
        keyboardUhid: true,
        mouseUhid: true,
        gamepadUhid: true,
      });
      onNotice('Started PC Gaming mode with UHID Keyboard, Mouse & Gamepad');
    } catch (e) {
      onError(String(e));
    }
  }

  async function mirrorDisplayId(id: number) {
    if (!serial) return;
    try {
      await mobileIpc.mirror(serial, {
        displayId: id,
        videoCodec,
        maxFps,
        bitRateMbps,
        audio,
        keyboardUhid,
        mouseUhid,
        gamepadUhid,
      });
      onNotice(`Mirrored Display #${id}`);
    } catch (e) {
      onError(String(e));
    }
  }
</script>

<div class="disp-layout">
  <section class="panel-box">
    <div class="bar">
      <div>
        <h3>Virtual Display &amp; Desktop Mode</h3>
        <p class="sub">
          Create an independent Android virtual screen on your PC while using your phone physically for something else.
        </p>
      </div>
      <button class="primary" disabled={!serial} onclick={launchVirtualDisplay}>
        <Icon name="monitor" size={14} /> Launch Virtual Display
      </button>
    </div>

    <div class="preset-row">
      <button class="preset" class:on={vdSize === '1920x1080'} onclick={() => pickPreset('1920x1080', 240)}>
        <strong>Desktop 1080p</strong>
        <span>1920×1080 · 240 dpi</span>
      </button>
      <button class="preset" class:on={vdSize === '2560x1080'} onclick={() => pickPreset('2560x1080', 240)}>
        <strong>Ultrawide</strong>
        <span>2560×1080 · 240 dpi</span>
      </button>
      <button class="preset" class:on={vdSize === '1600x1000'} onclick={() => pickPreset('1600x1000', 220)}>
        <strong>Tablet mode</strong>
        <span>1600×1000 · 220 dpi</span>
      </button>
      <button class="preset" class:on={vdSize === '880x1920'} onclick={() => pickPreset('880x1920', 320)}>
        <strong>Phone sidecar</strong>
        <span>880×1920 · 320 dpi</span>
      </button>
    </div>

    <div class="form-grid">
      <label>
        <span>Resolution (WxH)</span>
        <input type="text" bind:value={vdSize} placeholder="1920x1080" />
      </label>
      <label>
        <span>Density (DPI)</span>
        <input type="number" min="120" max="640" bind:value={vdDpi} />
      </label>
      <label>
        <span>Start app (optional package)</span>
        <input type="text" bind:value={startApp} placeholder="com.android.chrome" />
      </label>
      <label>
        <span>Video codec</span>
        <select bind:value={videoCodec}>
          <option value="h265">H.265 / HEVC</option>
          <option value="h264">H.264</option>
          <option value="av1">AV1</option>
        </select>
      </label>
      <label>
        <span>Max FPS</span>
        <select bind:value={maxFps}>
          <option value={30}>30 FPS</option>
          <option value={60}>60 FPS</option>
          <option value={120}>120 FPS</option>
        </select>
      </label>
      <label>
        <span>Bitrate (Mbps)</span>
        <select bind:value={bitRateMbps}>
          <option value={8}>8 Mbps</option>
          <option value={12}>12 Mbps</option>
          <option value={20}>20 Mbps</option>
          <option value={32}>32 Mbps</option>
        </select>
      </label>
    </div>

    <div class="toggles-grid">
      <div class="tg-item">
        <span>UHID Physical Keyboard (accents &amp; layouts)</span>
        <Switch checked={keyboardUhid} label="UHID Keyboard" onchange={(v) => (keyboardUhid = v)} />
      </div>
      <div class="tg-item">
        <span>UHID Physical Mouse cursor</span>
        <Switch checked={mouseUhid} label="UHID Mouse" onchange={(v) => (mouseUhid = v)} />
      </div>
      <div class="tg-item">
        <span>UHID PC Gamepad passthrough</span>
        <Switch checked={gamepadUhid} label="UHID Gamepad" onchange={(v) => (gamepadUhid = v)} />
      </div>
      <div class="tg-item">
        <span>Keep apps alive on phone when closed</span>
        <Switch checked={keepAliveOnClose} label="Keep apps alive" onchange={(v) => (keepAliveOnClose = v)} />
      </div>
      <div class="tg-item">
        <span>Hide Android system bars</span>
        <Switch checked={noDecorations} label="Hide system bars" onchange={(v) => (noDecorations = v)} />
      </div>
      <div class="tg-item">
        <span>Forward audio</span>
        <Switch checked={audio} label="Forward audio" onchange={(v) => (audio = v)} />
      </div>
      <div class="tg-item">
        <span>Turn phone screen off</span>
        <Switch checked={turnScreenOff} label="Turn phone screen off" onchange={(v) => (turnScreenOff = v)} />
      </div>
      <div class="tg-item">
        <span>Borderless / Always on top</span>
        <Switch
          checked={borderless && alwaysOnTop}
          label="Borderless and always on top"
          onchange={(v) => {
            borderless = v;
            alwaysOnTop = v;
          }}
        />
      </div>
    </div>
  </section>

  <div class="disp-side">
    <section class="panel-box">
      <h3>PC Gaming &amp; Controller Mode</h3>
      <p class="sub">
        Mirrors at native resolution and high bitrate with full hardware UHID Keyboard, Mouse and Xbox/PlayStation Gamepad passthrough into Android games.
      </p>
      <button class="primary" disabled={!serial} onclick={launchGamingMirror}>
        <Icon name="playtime" size={15} /> Start Gaming Mode (UHID)
      </button>
    </section>

    <section class="panel-box">
      <div class="bar">
        <h3>Active Phone Displays</h3>
        <button class="plain" disabled={!serial || loadingDisplays} onclick={() => void loadDisplays()}>
          <Icon name="refresh" size={14} />
        </button>
      </div>
      <p class="sub">Mirror your main screen, Samsung DeX, or any secondary display detected on the phone.</p>
      {#if !serial}
        <p class="sub">Connect your Android phone to detect displays.</p>
      {:else if displays.length === 0}
        <p class="sub">{loadingDisplays ? 'Checking displays…' : 'Only the default screen is active.'}</p>
      {:else}
        <ul class="disp-list">
          {#each displays as d (d.id)}
            <li>
              <span translate="no"><strong>{d.label}</strong></span>
              <button class="plain sm" onclick={() => void mirrorDisplayId(d.id)}>Mirror</button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
</div>

<style>
  .disp-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.45fr) minmax(280px, 0.9fr);
    gap: 14px;
    align-items: start;
  }
  @media (max-width: 920px) {
    .disp-layout { grid-template-columns: 1fr; }
  }
  .disp-side { display: grid; gap: 14px; }
  .panel-box {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    padding: 18px 20px;
    box-shadow: var(--shadow-card);
    display: grid;
    gap: 14px;
    align-content: start;
  }
  .bar { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  h3 { margin: 0; font-size: 15px; }
  .sub { margin: 2px 0 0; font-size: 12.5px; color: var(--text-3); line-height: 1.45; }

  .preset-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 8px; }
  .preset {
    display: grid; gap: 2px; text-align: left; padding: 9px 11px;
    border-radius: var(--radius-s); border: 1px solid var(--border); background: var(--surface-2);
    cursor: pointer;
  }
  .preset:hover, .preset.on { border-color: var(--accent); background: var(--surface-3); }
  .preset strong { font-size: 12.5px; }
  .preset span { font-size: 11px; color: var(--text-3); }

  .form-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(165px, 1fr)); gap: 10px; }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--text-3); font-weight: 600; }
  input[type='text'], input[type='number'], select {
    padding: 7px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: var(--text); font-size: 13px;
  }
  .toggles-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 8px 14px; }
  .tg-item {
    display: flex; align-items: center; justify-content: space-between; gap: 10px;
    padding: 7px 10px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border);
    font-size: 12.5px; font-weight: 500;
  }
  .disp-list { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  .disp-list li { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); font-size: 13px; }
  .plain, .primary {
    display: inline-flex; align-items: center; gap: 6px; padding: 7px 13px;
    border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: inherit; font-weight: 600; font-size: 12.5px; cursor: pointer;
  }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .plain.sm { padding: 4px 10px; font-size: 12px; }
  .primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .primary:hover:not(:disabled) { background: var(--accent-hover); }
  button:disabled { opacity: 0.45; cursor: default; }
</style>
