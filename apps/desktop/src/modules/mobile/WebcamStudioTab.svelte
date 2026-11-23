<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import { isLinux } from '../../lib/platform';
  import {
    mobileIpc,
    type MobileCameraCatalog,
    type MobileDevice,
    type MobileWebcamProOptions,
  } from '../../lib/ipc';

  let {
    device,
    webcamActive,
    onRefresh,
    onNotice,
    onError,
  }: {
    device: MobileDevice | null;
    webcamActive: boolean;
    onRefresh: () => void;
    onNotice: (msg: string) => void;
    onError: (msg: string) => void;
  } = $props();

  const serial = $derived(device?.state === 'device' ? device.serial : null);

  let catalog = $state<MobileCameraCatalog | null>(null);
  let probing = $state(false);
  let busy = $state(false);

  // Studio controls
  let cameraId = $state('');
  let facing = $state<'back' | 'front'>('back');
  let size = $state('1920x1080');
  let aspectRatio = $state('16:9');
  let fps = $state(60);
  let highSpeed = $state(false);
  let videoCodec = $state<'h264' | 'h265' | 'av1'>('h265');
  let bitRateMbps = $state(16);
  let orientation = $state(0);
  let flip = $state(false);
  let lockOrientation = $state(true);
  let audioMic = $state(false);
  let borderless = $state(true);
  let alwaysOnTop = $state(false);
  let record = $state(false);
  let v4l2Sink = $state('');

  const selectedCamera = $derived(
    catalog?.cameras.find((c) => c.id === cameraId) ?? catalog?.cameras[0] ?? null,
  );
  const availableSizes = $derived(() => {
    const base = selectedCamera?.sizes ?? [];
    const fallback = ['3840x2160', '2560x1440', '1920x1080', '1280x720'];
    return base.length > 0 ? base : fallback;
  });

  async function probeCameras() {
    if (!serial) return;
    probing = true;
    try {
      catalog = await mobileIpc.cameras(serial);
      if (catalog.cameras.length > 0 && !cameraId) {
        cameraId = catalog.cameras[0].id;
        facing = catalog.cameras[0].facing === 'front' ? 'front' : 'back';
        const has1080 = catalog.cameras[0].sizes.find((s) => s === '1920x1080');
        if (has1080) size = has1080;
        else if (catalog.cameras[0].sizes[0]) size = catalog.cameras[0].sizes[0];
      }
      if (catalog.v4l2_devices.length > 0 && !v4l2Sink) {
        v4l2Sink = catalog.v4l2_devices[catalog.v4l2_devices.length - 1];
      }
    } catch (e) {
      onError(String(e));
    } finally {
      probing = false;
    }
  }

  $effect(() => {
    if (serial) void probeCameras();
  });

  function applyPreset(name: 'stream' | '4k' | 'calls' | 'vertical' | 'highspeed') {
    if (name === 'stream') {
      size = '1920x1080';
      aspectRatio = '16:9';
      fps = 60;
      highSpeed = false;
      videoCodec = 'h265';
      bitRateMbps = 16;
      orientation = 0;
      borderless = true;
    } else if (name === '4k') {
      size = '3840x2160';
      aspectRatio = '16:9';
      fps = 30;
      highSpeed = false;
      videoCodec = 'h265';
      bitRateMbps = 28;
      orientation = 0;
      borderless = true;
    } else if (name === 'calls') {
      size = '1280x720';
      aspectRatio = '16:9';
      fps = 30;
      highSpeed = false;
      videoCodec = 'h264';
      bitRateMbps = 6;
      orientation = 0;
      flip = true;
    } else if (name === 'vertical') {
      size = '1920x1080';
      aspectRatio = '9:16';
      fps = 60;
      highSpeed = false;
      videoCodec = 'h265';
      bitRateMbps = 14;
      orientation = 90;
    } else if (name === 'highspeed') {
      size = '1920x1080';
      aspectRatio = '16:9';
      fps = 120;
      highSpeed = true;
      videoCodec = 'h265';
      bitRateMbps = 24;
    }
    onNotice(`Applied ${name.toUpperCase()} camera profile`);
  }

  async function startOrRestart() {
    if (!serial) return;
    busy = true;
    try {
      const opts: MobileWebcamProOptions = {
        cameraId: cameraId || undefined,
        facing,
        size: size || undefined,
        aspectRatio,
        fps,
        highSpeed,
        videoCodec,
        bitRateMbps,
        orientation,
        flip,
        lockOrientation,
        audioMic,
        borderless,
        alwaysOnTop,
        record,
        v4l2Sink: isLinux && v4l2Sink ? v4l2Sink : undefined,
      };
      await mobileIpc.webcamPro(serial, opts, false);
      onNotice(
        `Webcam Studio running (${size || aspectRatio} · ${fps}fps · ${videoCodec.toUpperCase()})`,
      );
      onRefresh();
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function stopWebcam() {
    if (!serial) return;
    busy = true;
    try {
      await mobileIpc.webcamPro(serial, undefined, true);
      onNotice('Webcam Studio stopped');
      onRefresh();
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function switchNativeUvc() {
    if (!serial) return;
    busy = true;
    try {
      onNotice(await mobileIpc.usbWebcam(serial));
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function openObs() {
    try {
      onNotice(await mobileIpc.launchObs());
    } catch (e) {
      onError(String(e));
    }
  }
</script>

<div class="cam-layout">
  <section class="panel-box">
    <div class="bar">
      <div>
        <h3>Webcam Studio</h3>
        <p class="sub">
          Full sensor access up to 4K, 60/120 FPS, H.265/AV1, multi-lens switching and zero watermarks (Android 12+).
        </p>
      </div>
      <div class="bar-acts">
        <button class="plain" disabled={!serial || probing} onclick={() => void probeCameras()}>
          <Icon name="refresh" size={14} /> {probing ? 'Detecting lenses…' : 'Detect lenses'}
        </button>
        {#if webcamActive}
          <button class="plain danger-btn" disabled={busy} onclick={stopWebcam}>
            <Icon name="square" size={13} /> Stop
          </button>
          <button class="primary" disabled={!serial || busy} onclick={startOrRestart}>
            <Icon name="video" size={14} /> Apply &amp; Restart
          </button>
        {:else}
          <button class="primary" disabled={!serial || busy} onclick={startOrRestart}>
            <Icon name="video" size={14} /> Start Webcam
          </button>
        {/if}
      </div>
    </div>

    <!-- 1-click Pro Presets -->
    <div class="preset-row">
      <button class="preset" onclick={() => applyPreset('stream')}>
        <strong>Streaming 1080p60</strong>
        <span>H.265 · 16 Mbps · Borderless</span>
      </button>
      <button class="preset" onclick={() => applyPreset('4k')}>
        <strong>Cinema 4K UHD</strong>
        <span>3840×2160 · 30fps · 28 Mbps</span>
      </button>
      <button class="preset" onclick={() => applyPreset('calls')}>
        <strong>Calls &amp; Discord</strong>
        <span>720p30 · Low latency · Flipped</span>
      </button>
      <button class="preset" onclick={() => applyPreset('vertical')}>
        <strong>Vertical 9:16</strong>
        <span>60fps · Rotated 90°</span>
      </button>
      <button class="preset" onclick={() => applyPreset('highspeed')}>
        <strong>High-Speed 120fps</strong>
        <span>Sensor high-speed mode</span>
      </button>
    </div>

    <div class="form-grid">
      <label>
        <span>Camera lens</span>
        {#if catalog && catalog.cameras.length > 0}
          <select
            bind:value={cameraId}
            onchange={() => {
              const found = catalog?.cameras.find((c) => c.id === cameraId);
              if (found) {
                facing = found.facing === 'front' ? 'front' : 'back';
                if (found.sizes.length > 0 && !found.sizes.includes(size)) {
                  size = found.sizes.find((s) => s === '1920x1080') ?? found.sizes[0];
                }
              }
            }}
          >
            {#each catalog.cameras as cam (cam.id)}
              <option value={cam.id} translate="no">{cam.label}</option>
            {/each}
          </select>
        {:else}
          <select bind:value={facing}>
            <option value="back">Back main camera</option>
            <option value="front">Front selfie camera</option>
          </select>
        {/if}
      </label>

      <label>
        <span>Resolution</span>
        <select bind:value={size}>
          <option value="">Auto (match aspect ratio)</option>
          {#each availableSizes() as sz (sz)}
            <option value={sz} translate="no">{sz}</option>
          {/each}
        </select>
      </label>

      <label>
        <span>Frame rate</span>
        <select bind:value={fps}>
          <option value={24}>24 FPS (Cinema)</option>
          <option value={30}>30 FPS</option>
          <option value={60}>60 FPS (Smooth)</option>
          <option value={120}>120 FPS (High-speed)</option>
        </select>
      </label>

      <label>
        <span>Video codec</span>
        <select bind:value={videoCodec}>
          <option value="h265">H.265 / HEVC (Sharpest)</option>
          <option value="h264">H.264 (Widest support)</option>
          <option value="av1">AV1 (Modern hardware)</option>
        </select>
      </label>

      <label>
        <span>Aspect ratio</span>
        <select bind:value={aspectRatio}>
          <option value="16:9">16:9 Widescreen</option>
          <option value="4:3">4:3 Classic</option>
          <option value="1:1">1:1 Square</option>
          <option value="9:16">9:16 Vertical</option>
          <option value="sensor">Sensor native</option>
        </select>
      </label>

      <label>
        <span>Rotation</span>
        <select bind:value={orientation}>
          <option value={0}>0° (Normal)</option>
          <option value={90}>90° clockwise</option>
          <option value={180}>180° upside down</option>
          <option value={270}>270° counter-clockwise</option>
        </select>
      </label>
    </div>

    <div class="slider-block">
      <div class="sl-head">
        <span>Video bitrate (higher = cleaner darks &amp; sharper detail)</span>
        <strong class="tabular">{bitRateMbps} Mbps</strong>
      </div>
      <input type="range" min="4" max="32" step="2" bind:value={bitRateMbps} />
    </div>

    <div class="toggles-grid">
      <div class="tg-item">
        <span>Mirror / Flip horizontally</span>
        <Switch checked={flip} label="Mirror horizontally" onchange={(v) => (flip = v)} />
      </div>
      <div class="tg-item">
        <span>Lock capture orientation</span>
        <Switch checked={lockOrientation} label="Lock capture orientation" onchange={(v) => (lockOrientation = v)} />
      </div>
      <div class="tg-item">
        <span>High-speed sensor mode</span>
        <Switch checked={highSpeed} label="High-speed sensor mode" onchange={(v) => (highSpeed = v)} />
      </div>
      <div class="tg-item">
        <span>Include phone microphone</span>
        <Switch checked={audioMic} label="Include phone microphone" onchange={(v) => (audioMic = v)} />
      </div>
      <div class="tg-item">
        <span>Borderless clean window</span>
        <Switch checked={borderless} label="Borderless window" onchange={(v) => (borderless = v)} />
      </div>
      <div class="tg-item">
        <span>Always on top</span>
        <Switch checked={alwaysOnTop} label="Always on top" onchange={(v) => (alwaysOnTop = v)} />
      </div>
      <div class="tg-item">
        <span>Record camera to MKV file</span>
        <Switch checked={record} label="Record camera to MKV" onchange={(v) => (record = v)} />
      </div>
    </div>
  </section>

  <!-- Right Column: Virtual Webcam Integration -->
  <div class="cam-side">
    <section class="panel-box">
      <h3>Use in Discord, Zoom, Meet &amp; OBS</h3>

      {#if isLinux}
        <div class="bridge-card">
          <strong>Linux Virtual Webcam (v4l2loopback)</strong>
          <p class="sub">
            Feed your phone directly into a real <code>/dev/video*</code> webcam device so Discord, Zoom and browsers pick it up natively.
          </p>
          <label>
            <span>Virtual device sink</span>
            <select bind:value={v4l2Sink}>
              <option value="">Window preview only</option>
              {#each catalog?.v4l2_devices ?? [] as dev (dev)}
                <option value={dev} translate="no">{dev}</option>
              {/each}
            </select>
          </label>
          <code class="cmd-chip">sudo modprobe v4l2loopback exclusive_caps=1 card_label="NomNom Cam"</code>
        </div>
      {:else}
        <div class="bridge-card">
          <div class="bhead">
            <strong>Option 1 · Native Android 14+ USB Webcam</strong>
            {#if catalog?.supports_usb_uvc}
              <span class="pill ok">Supported (API {catalog.sdk_level})</span>
            {:else}
              <span class="pill">Android 14+</span>
            {/if}
          </div>
          <p class="sub">
            Switches your phone into a hardware USB UVC webcam recognized natively by Windows, Discord, Zoom and Meet with zero extra drivers.
          </p>
          <button
            class="primary"
            disabled={!serial || busy || device?.transport !== 'usb'}
            onclick={switchNativeUvc}
          >
            <Icon name="usb" size={14} /> Enable Hardware USB Webcam
          </button>
          {#if serial && device?.transport !== 'usb'}
            <p class="sub">Plug in a USB cable to use hardware UVC mode.</p>
          {/if}
        </div>

        <div class="bridge-card">
          <div class="bhead">
            <strong>Option 2 · OBS Virtual Camera (Any Android 12+, USB or Wi-Fi)</strong>
            {#if catalog?.obs_path}
              <span class="pill ok">OBS detected</span>
            {/if}
          </div>
          <ol class="mini-steps">
            <li>Click <strong>Start Webcam</strong> (borderless window titled <code>NomNom Webcam</code>).</li>
            <li>In OBS Studio, add <strong>Window Capture</strong> → pick <code>NomNom Webcam</code>.</li>
            <li>Click <strong>Start Virtual Camera</strong> in OBS and select <strong>OBS Virtual Camera</strong> in Discord, Zoom or Meet.</li>
          </ol>
          {#if catalog?.obs_path}
            <button class="plain" onclick={openObs}>
              <Icon name="external" size={14} /> Launch OBS Studio
            </button>
          {/if}
        </div>
      {/if}
    </section>

    {#if selectedCamera && selectedCamera.high_speed.length > 0}
      <section class="panel-box">
        <h3>High-speed modes on this lens</h3>
        <div class="hs-tags">
          {#each selectedCamera.high_speed as hs (hs)}
            <span class="pill" translate="no">{hs}</span>
          {/each}
        </div>
      </section>
    {/if}
  </div>
</div>

<style>
  .cam-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.45fr) minmax(290px, 0.95fr);
    gap: 14px;
    align-items: start;
  }
  @media (max-width: 940px) {
    .cam-layout { grid-template-columns: 1fr; }
  }
  .cam-side { display: grid; gap: 14px; }
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
  .bar-acts { display: flex; gap: 8px; flex-wrap: wrap; }
  h3 { margin: 0; font-size: 15px; }
  .sub { margin: 2px 0 0; font-size: 12.5px; color: var(--text-3); line-height: 1.45; }

  .preset-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(145px, 1fr)); gap: 8px; }
  .preset {
    display: grid; gap: 2px; text-align: left; padding: 9px 11px;
    border-radius: var(--radius-s); border: 1px solid var(--border); background: var(--surface-2);
    cursor: pointer; transition: border-color var(--dur-1), background var(--dur-1);
  }
  .preset:hover { border-color: var(--accent); background: var(--surface-3); }
  .preset strong { font-size: 12.5px; color: var(--text); }
  .preset span { font-size: 11px; color: var(--text-3); }

  .form-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px; }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--text-3); font-weight: 600; }
  select {
    padding: 7px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: var(--text); font-size: 13px;
  }
  .slider-block { display: grid; gap: 5px; }
  .sl-head { display: flex; justify-content: space-between; font-size: 12.5px; color: var(--text-2); }
  input[type='range'] { width: 100%; accent-color: var(--accent); }

  .toggles-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 8px 16px; }
  .tg-item {
    display: flex; align-items: center; justify-content: space-between; gap: 10px;
    padding: 7px 10px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border);
    font-size: 12.5px; font-weight: 500;
  }

  .bridge-card {
    display: grid; gap: 8px; padding: 12px 14px; border-radius: var(--radius-m);
    background: var(--surface-2); border: 1px solid var(--border);
  }
  .bhead { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; font-size: 13px; }
  .pill { padding: 2px 8px; border-radius: var(--radius-xs); background: var(--surface-3); color: var(--text-2); font-size: 11px; font-weight: 600; }
  .pill.ok { color: var(--ok); }
  .mini-steps { margin: 0; padding-left: 18px; display: grid; gap: 4px; font-size: 12.5px; color: var(--text-2); line-height: 1.45; }
  .cmd-chip { font-family: var(--font-mono); font-size: 11.5px; padding: 7px 9px; border-radius: var(--radius-xs); background: var(--surface-1); border: 1px solid var(--border); user-select: all; word-break: break-all; }
  .hs-tags { display: flex; flex-wrap: wrap; gap: 6px; }

  .plain, .primary {
    display: inline-flex; align-items: center; gap: 6px; padding: 7px 13px;
    border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: inherit; font-weight: 600; font-size: 12.5px; cursor: pointer;
  }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .primary:hover:not(:disabled) { background: var(--accent-hover); }
  .danger-btn { color: var(--danger); }
  button:disabled { opacity: 0.45; cursor: default; }
</style>
