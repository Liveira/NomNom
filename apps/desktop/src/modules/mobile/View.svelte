<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { openPath } from '@tauri-apps/plugin-opener';
  import { onDestroy } from 'svelte';
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { every } from '../../lib/poll';
  import {
    mobileIpc,
    onMobileInstallProgress,
    onMobileTimeline,
    phoneIpc,
    type MobileCapturedImage,
    type MobileDevice,
    type MobileMirrorOptions,
    type MobileNotification,
    type MobileSnapshot,
    type MobileSoundDisplayPatch,
    type MobileTimelineEntry,
    type MobileToolsStatus,
  } from '../../lib/ipc';
  import DisplayGamingTab from './DisplayGamingTab.svelte';
  import HomeTab from './HomeTab.svelte';
  import Modals from './Modals.svelte';
  import PowerTabs from './PowerTabs.svelte';
  import SubTabs from './SubTabs.svelte';
  import WebcamStudioTab from './WebcamStudioTab.svelte';
  import type { MobileTab } from './logic';

  const TABS: { id: MobileTab; label: string; icon: string }[] = [
    { id: 'home', label: 'Home', icon: 'home' },
    { id: 'webcam', label: 'Webcam', icon: 'video' },
    { id: 'display', label: 'Display', icon: 'monitor' },
    { id: 'messages', label: 'Messages', icon: 'message' },
    { id: 'calls', label: 'Calls', icon: 'phone' },
    { id: 'photos', label: 'Photos', icon: 'image' },
    { id: 'files', label: 'Files', icon: 'folder' },
    { id: 'apps', label: 'Apps', icon: 'grid' },
    { id: 'system', label: 'System', icon: 'cpu' },
    { id: 'console', label: 'Console', icon: 'terminal' },
    { id: 'deck', label: 'Deck', icon: 'layers' },
    { id: 'guide', label: 'Guide', icon: 'book' },
    { id: 'settings', label: 'Settings', icon: 'settings' },
  ];

  let tab = $state<MobileTab>('home');
  let tools = $state<MobileToolsStatus | null>(null);
  let installProgress = $state<{ done: number; total: number | null } | null>(null);
  let devices = $state<MobileDevice[]>([]);
  let selectedSerial = $state<string | null>(null);
  let snapshot = $state<MobileSnapshot | null>(null);
  let timeline = $state<MobileTimelineEntry[]>([]);
  let modal = $state<'none' | 'pair' | 'sound' | 'timeline' | 'commands' | 'capture'>('none');
  let captured = $state<MobileCapturedImage | null>(null);
  let busyAction = $state('');
  let refreshing = $state(false);
  let notice = $state('');
  let error = $state('');
  let noticeTimer: ReturnType<typeof setTimeout> | null = null;

  let mirrorOpts = $state<MobileMirrorOptions>({
    maxSize: 1280,
    bitRateMbps: 8,
    maxFps: 60,
    turnScreenOff: false,
    stayAwake: true,
    audio: true,
  });

  const activeDevice = $derived(
    devices.find((d) => d.serial === selectedSerial) ??
      devices.find((d) => d.state === 'device') ??
      devices[0] ??
      null,
  );
  const currentTabTitle = $derived(TABS.find((t) => t.id === tab)?.label ?? 'Home');
  const needsTools = $derived(tools !== null && (!tools.adb_found || !tools.scrcpy_found));

  function showNotice(msg: string) {
    notice = msg;
    error = '';
    if (noticeTimer) clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (notice = ''), 4000);
  }
  function showError(msg: string) {
    error = msg;
  }

  let autoTried = false;

  async function refreshAll(silent = false) {
    if (!silent) refreshing = true;
    try {
      let [t, devs, tl] = await Promise.all([
        mobileIpc.toolsStatus(),
        mobileIpc.devices().catch(() => []),
        mobileIpc.timeline(40).catch(() => []),
      ]);
      // On first load, if tools are ready and no phone is connected yet, quietly auto-reconnect known/mDNS Wi-Fi phones.
      if (!autoTried && t.adb_found && devs.filter((d) => d.state === 'device').length === 0) {
        autoTried = true;
        const found = await mobileIpc.wifiAutoconnect().catch(() => []);
        if (found.length > 0) {
          devs = await mobileIpc.devices().catch(() => devs);
          showNotice(`Auto-connected over Wi-Fi (${found[0]})`);
        }
      }
      tools = t;
      devices = devs;
      timeline = tl;

      const pick =
        devs.find((d) => d.serial === selectedSerial) ??
        devs.find((d) => d.state === 'device') ??
        devs[0] ??
        null;
      selectedSerial = pick?.serial ?? null;

      if (pick && pick.state === 'device') {
        snapshot = await mobileIpc.snapshot(pick.serial).catch(() => snapshot);
      } else {
        snapshot = null;
      }
    } catch (e) {
      if (!silent) showError(String(e));
    } finally {
      if (!silent) refreshing = false;
    }
  }

  void refreshAll();
  const poll = every(6500, () => {
    if (!document.hidden && !tools?.installing) void refreshAll(true);
  });
  let unsubs: (() => void)[] = [];
  let gone = false;
  const track = (p: Promise<() => void>) =>
    void p.then((u) => {
      if (gone) u();
      else unsubs.push(u);
    });
  track(onMobileInstallProgress((p) => (installProgress = p)));
  track(
    onMobileTimeline(() => {
      void mobileIpc.timeline(40).then((tl) => (timeline = tl)).catch(() => {});
    }),
  );

  onDestroy(() => {
    gone = true;
    poll();
    if (noticeTimer) clearTimeout(noticeTimer);
    unsubs.forEach((u) => u());
  });

  async function installTools() {
    error = '';
    installProgress = { done: 0, total: null };
    try {
      tools = await mobileIpc.installTools();
      installProgress = null;
      showNotice('ADB and scrcpy installed.');
      await refreshAll();
    } catch (e) {
      installProgress = null;
      showError(String(e));
    }
  }

  async function runMirror(startApp?: string, displaySize?: string) {
    if (!activeDevice || activeDevice.state !== 'device') return;
    busyAction = 'mirror';
    try {
      await mobileIpc.mirror(activeDevice.serial, {
        ...mirrorOpts,
        ...(startApp ? { startApp, newDisplay: true, displaySize } : {}),
      });
      showNotice(startApp ? 'Opened app window' : 'Screen mirroring started');
    } catch (e) {
      showError(String(e));
    } finally {
      busyAction = '';
    }
  }

  async function sendFiles() {
    if (!activeDevice || activeDevice.state !== 'device') return;
    const picked = await open({ multiple: true });
    if (!picked) return;
    busyAction = 'files';
    try {
      const list = Array.isArray(picked) ? picked : [picked];
      const count = await mobileIpc.sendFiles(activeDevice.serial, list);
      showNotice(`Sent ${count} ${count === 1 ? 'file' : 'files'} to /Download/NomNom`);
    } catch (e) {
      showError(String(e));
    } finally {
      busyAction = '';
    }
  }

  async function openSavedFolder() {
    try {
      const st = await phoneIpc.status();
      if (st.dest) await openPath(st.dest);
    } catch {
      app.go('phone');
    }
  }

  async function sendClipboard() {
    if (!activeDevice || activeDevice.state !== 'device') return;
    busyAction = 'clipboard';
    try {
      showNotice(await mobileIpc.sendClipboard(activeDevice.serial, undefined, true));
    } catch (e) {
      showError(String(e));
    } finally {
      busyAction = '';
    }
  }

  async function findPhone() {
    if (!activeDevice || activeDevice.state !== 'device') return;
    busyAction = 'find';
    try {
      showNotice(await mobileIpc.findPhone(activeDevice.serial));
    } catch (e) {
      showError(String(e));
    } finally {
      busyAction = '';
    }
  }

  async function mediaAction(act: 'play_pause' | 'next' | 'prev') {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      await mobileIpc.mediaAction(activeDevice.serial, act);
      setTimeout(() => void refreshAll(true), 450);
    } catch (e) {
      showError(String(e));
    }
  }

  async function toggleControl(
    ctrl: 'dnd' | 'torch' | 'wifi' | 'bluetooth' | 'hotspot',
    next: boolean,
  ) {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      const msg = await mobileIpc.toggleControl(activeDevice.serial, ctrl, next);
      showNotice(msg);
      await refreshAll(true);
    } catch (e) {
      showError(String(e));
    }
  }

  async function toggleAudio(next: boolean) {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      const active = await mobileIpc.audioToggle(activeDevice.serial, next);
      showNotice(active ? 'Phone audio bridge active' : 'Phone audio bridge stopped');
      await refreshAll(true);
    } catch (e) {
      showError(String(e));
    }
  }

  async function toggleMic(next: boolean) {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      const active = await mobileIpc.micToggle(activeDevice.serial, next);
      showNotice(active ? 'Phone microphone stream active' : 'Phone microphone stopped');
      await refreshAll(true);
    } catch (e) {
      showError(String(e));
    }
  }

  async function toggleRecord() {
    if (!activeDevice || activeDevice.state !== 'device') return;
    const currentlyRec = snapshot?.controls.recording_active ?? false;
    try {
      const saved = await mobileIpc.recordToggle(activeDevice.serial, !currentlyRec);
      if (saved) {
        showNotice(`Recording saved: ${saved}`);
        void openPath(saved).catch(() => {});
      } else {
        showNotice('Recording phone screen to MKV…');
      }
      await refreshAll(true);
    } catch (e) {
      showError(String(e));
    }
  }

  async function runSwipe(dir: 'up' | 'down' | 'left' | 'right' | 'notifications' | 'quicksettings') {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      await mobileIpc.swipe(activeDevice.serial, dir);
    } catch (e) {
      showError(String(e));
    }
  }

  async function continuityCamera() {
    if (!activeDevice || activeDevice.state !== 'device') return;
    busyAction = 'camera';
    showNotice('Camera opened on your phone — take a photo to bring it here.');
    try {
      captured = await mobileIpc.continuityCamera(activeDevice.serial);
      modal = 'capture';
    } catch (e) {
      showError(String(e));
    } finally {
      busyAction = '';
    }
  }

  async function toggleWebcam(facing: 'back' | 'front' = 'back') {
    if (!activeDevice || activeDevice.state !== 'device') return;
    busyAction = 'webcam';
    try {
      const on = await mobileIpc.webcam(activeDevice.serial, facing);
      showNotice(on ? `Phone webcam (${facing}) started` : 'Phone webcam stopped');
      await refreshAll(true);
    } catch (e) {
      showError(String(e));
    } finally {
      busyAction = '';
    }
  }

  async function takeScreenshot() {
    if (!activeDevice || activeDevice.state !== 'device') return;
    busyAction = 'camera';
    try {
      captured = await mobileIpc.screenshot(activeDevice.serial);
      modal = 'capture';
    } catch (e) {
      showError(String(e));
    } finally {
      busyAction = '';
    }
  }

  async function typeText(text: string) {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      await mobileIpc.typeText(activeDevice.serial, text);
      await mobileIpc.keyevent(activeDevice.serial, 'enter');
    } catch (e) {
      showError(String(e));
    }
  }

  async function sendKey(key: string) {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      await mobileIpc.keyevent(activeDevice.serial, key);
    } catch (e) {
      showError(String(e));
    }
  }

  async function notificationAction(
    n: MobileNotification | null,
    act: 'dismiss' | 'clear_all' | 'mute' | 'open' | 'reply',
    replyText?: string,
  ) {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      const msg = await mobileIpc.notificationAction(
        activeDevice.serial,
        n?.package ?? '',
        n?.key ?? '',
        act,
        replyText,
      );
      showNotice(msg);
      await refreshAll(true);
    } catch (e) {
      showError(String(e));
    }
  }

  async function unmuteApp(pkg: string) {
    if (!activeDevice) return;
    try {
      showNotice(await mobileIpc.notificationAction(activeDevice.serial, pkg, '', 'unmute'));
      await refreshAll(true);
    } catch (e) {
      showError(String(e));
    }
  }

  async function applySoundDisplay(patch: MobileSoundDisplayPatch) {
    if (!activeDevice || activeDevice.state !== 'device') return;
    try {
      await mobileIpc.setSoundDisplay(activeDevice.serial, patch);
      await refreshAll(true);
    } catch (e) {
      showError(String(e));
    }
  }

  const COMMANDS = [
    { id: 'mirror', label: 'Mirror screen', hint: 'Open live phone screen window via scrcpy', icon: 'cast' },
    { id: 'record', label: 'Record phone screen', hint: 'Start or stop recording the phone display to MKV', icon: 'record' },
    { id: 'files', label: 'Send files', hint: 'Push files from this computer to /Download/NomNom', icon: 'send' },
    { id: 'clipboard', label: 'Send clipboard', hint: 'Send text or open copied URL on your phone', icon: 'clipboard' },
    { id: 'find', label: 'Find phone', hint: 'Wake, unmute and ring your phone', icon: 'bell-ring' },
    { id: 'camera', label: 'Continuity Camera', hint: 'Open camera on phone and pull the photo here', icon: 'camera' },
    { id: 'webcam-studio', label: 'Open Webcam Studio', hint: '4K/60fps/120fps, multi-lens, H.265/AV1, USB UVC & OBS', icon: 'video' },
    { id: 'virtual-display', label: 'Open Virtual Display & Gaming', hint: 'Second monitor, Desktop mode & UHID Keyboard/Mouse/Gamepad', icon: 'monitor' },
    { id: 'webcam', label: 'Phone as webcam', hint: 'Quick-start your phone camera (Android 12+)', icon: 'video' },
    { id: 'mic', label: 'Phone as microphone', hint: 'Stream phone microphone audio to this computer', icon: 'mic' },
    { id: 'screenshot', label: 'Phone screenshot', hint: 'Capture the phone screen immediately', icon: 'image' },
    { id: 'dark', label: 'Toggle phone dark mode', hint: 'Switch Android light/dark theme right away', icon: 'moon' },
    { id: 'sound', label: 'Sound & display', hint: 'Adjust volumes, brightness and screen timeout', icon: 'sliders' },
    { id: 'pair', label: 'Pair over Wi-Fi', hint: 'Connect an Android phone without USB cable', icon: 'wifi' },
    { id: 'tab-files', label: 'Open Phone Files explorer', hint: 'Browse, upload and download /sdcard files', icon: 'folder' },
    { id: 'tab-system', label: 'Open System & Tweaks', hint: 'Hardware, RAM, DPI, animations and reboot options', icon: 'cpu' },
    { id: 'tab-console', label: 'Open ADB Shell & Logcat', hint: 'Run commands, read logcat and set port tunnels', icon: 'terminal' },
    { id: 'lock', label: 'Lock phone screen', hint: 'Put the phone display to sleep', icon: 'lock' },
    { id: 'wake', label: 'Wake phone screen', hint: 'Turn the phone display on', icon: 'sun' },
  ];

  function runCommand(id: string) {
    switch (id) {
      case 'mirror': void runMirror(); break;
      case 'record': void toggleRecord(); break;
      case 'files': void sendFiles(); break;
      case 'clipboard': void sendClipboard(); break;
      case 'find': void findPhone(); break;
      case 'camera': void continuityCamera(); break;
      case 'webcam-studio': tab = 'webcam'; break;
      case 'virtual-display': tab = 'display'; break;
      case 'webcam': void toggleWebcam('back'); break;
      case 'mic': void toggleMic(!(snapshot?.controls.mic_active ?? false)); break;
      case 'screenshot': void takeScreenshot(); break;
      case 'dark':
        if (activeDevice && activeDevice.state === 'device') {
          void mobileIpc
            .setTweak(activeDevice.serial, 'dark_mode', snapshot?.controls.dark_mode ? 'off' : 'on')
            .then((m) => {
              showNotice(m);
              void refreshAll(true);
            })
            .catch((e) => showError(String(e)));
        }
        break;
      case 'sound': modal = 'sound'; break;
      case 'pair': modal = 'pair'; break;
      case 'tab-files': tab = 'files'; break;
      case 'tab-system': tab = 'system'; break;
      case 'tab-console': tab = 'console'; break;
      case 'lock': void sendKey('sleep'); break;
      case 'wake': void sendKey('wakeup'); break;
    }
  }
</script>

<div class="mob-shell">
  <!-- Compact vertical rail (matches reference layout) -->
  <aside class="mob-rail" aria-label="Mobile Manager navigation">
    <div class="rail-brand" title="NomNom Mobile Manager">
      <Icon name="smartphone" size={18} />
    </div>

    <div class="rail-tabs" role="tablist" aria-orientation="vertical">
      {#each TABS as t (t.id)}
        <button
          role="tab"
          aria-selected={tab === t.id}
          class="rail-btn"
          class:on={tab === t.id}
          onclick={() => (tab = t.id)}
        >
          <span class="r-ico"><Icon name={t.icon} size={17} /></span>
          <span class="r-lbl">{t.label}</span>
        </button>
      {/each}
    </div>

    <button
      class="rail-add"
      title="Connect or pair a phone"
      aria-label="Connect or pair a phone"
      onclick={() => (modal = 'pair')}
    >
      <Icon name="plus" size={17} />
    </button>
  </aside>

  <!-- Main Workspace -->
  <div class="mob-main">
    <header class="mob-topbar">
      <div class="tb-left">
        <h1>{currentTabTitle}</h1>
        {#if devices.length > 1}
          <select
            class="dev-pick"
            aria-label="Select phone"
            value={selectedSerial ?? ''}
            onchange={(e) => {
              selectedSerial = e.currentTarget.value;
              void refreshAll();
            }}
          >
            {#each devices as d (d.serial)}
              <option value={d.serial} translate="no">{d.model} ({d.transport.toUpperCase()})</option>
            {/each}
          </select>
        {/if}
      </div>

      <div class="tb-right">
        {#if activeDevice?.state === 'device' && activeDevice.transport === 'usb'}
          <button
            class="tb-btn"
            title="Enable wireless ADB and unplug USB"
            onclick={async () => {
              try {
                const ep = await mobileIpc.enableWifi(activeDevice.serial);
                showNotice(`Switched to Wi-Fi at ${ep} — you can unplug USB now.`);
                await refreshAll();
              } catch (e) {
                showError(String(e));
              }
            }}
          >
            <Icon name="wifi" size={14} /> Go Wireless
          </button>
        {:else}
          <button class="tb-btn" onclick={() => (modal = 'pair')}>
            <Icon name="wifi" size={14} /> {activeDevice?.transport === 'wifi' ? 'Wi-Fi Connected' : 'Connect Wi-Fi (QR)'}
          </button>
        {/if}
        <button class="tb-btn" onclick={() => (modal = 'commands')}>
          <Icon name="search" size={14} /> Commands
        </button>
        <button class="tb-btn" onclick={() => (modal = 'timeline')}>
          <Icon name="clock" size={14} /> Timeline
        </button>
        <button
          class="tb-ico"
          class:spin={refreshing}
          title="Refresh device status"
          aria-label="Refresh device status"
          onclick={() => void refreshAll()}
        >
          <Icon name="refresh" size={15} />
        </button>
      </div>
    </header>

    {#if needsTools}
      <div class="setup-banner">
        <div>
          <strong>Android tools needed (ADB + scrcpy)</strong>
          <p>Download the official scrcpy package (~30 MB) once to enable mirroring, notifications, controls, photos and Wi-Fi pairing.</p>
          {#if installProgress}
            <div class="prog-wrap">
              <ProgressBar
                value={installProgress.total ? installProgress.done / installProgress.total : null}
                label="Installing ADB and scrcpy"
              />
            </div>
          {/if}
        </div>
        <button class="primary" disabled={tools?.installing} onclick={installTools}>
          <Icon name="cloud-download" size={15} />
          {tools?.installing ? 'Installing…' : 'Install Android tools'}
        </button>
      </div>
    {/if}

    {#if notice}
      <div class="banner ok" role="status">{notice}</div>
    {/if}
    {#if error}
      <div class="banner err" role="alert">
        <span>{error}</span>
        <button class="xbtn" aria-label="Dismiss error" onclick={() => (error = '')}>
          <Icon name="x" size={13} />
        </button>
      </div>
    {/if}

    {#if tab === 'home'}
      <HomeTab
        device={activeDevice}
        {snapshot}
        {timeline}
        {busyAction}
        onMirror={() => void runMirror()}
        onOpenAppPicker={() => (tab = 'apps')}
        onSendFiles={() => void sendFiles()}
        onOpenSavedFolder={() => void openSavedFolder()}
        onSendClipboard={() => void sendClipboard()}
        onFindPhone={() => void findPhone()}
        onMediaAction={(act) => void mediaAction(act)}
        onToggleControl={(ctrl, next) => void toggleControl(ctrl, next)}
        onAudioToggle={(next) => void toggleAudio(next)}
        onMicToggle={(next) => void toggleMic(next)}
        onRecordToggle={() => void toggleRecord()}
        onSwipe={(dir) => void runSwipe(dir)}
        onOpenSoundDisplay={() => (modal = 'sound')}
        onContinuityCamera={() => void continuityCamera()}
        onWebcam={(facing) => void toggleWebcam(facing)}
        onTypeText={typeText}
        onKeyEvent={(k) => void sendKey(k)}
        onNotificationAction={(n, a, r) => void notificationAction(n, a, r)}
        onOpenHistory={() => (modal = 'timeline')}
        onOpenPair={() => (modal = 'pair')}
      />
    {:else if tab === 'webcam'}
      <WebcamStudioTab
        device={activeDevice}
        webcamActive={snapshot?.controls.webcam_active ?? false}
        onRefresh={() => void refreshAll(true)}
        onOpenPair={() => (modal = 'pair')}
        onGoWireless={async () => {
          if (!activeDevice) return;
          try {
            const ep = await mobileIpc.enableWifi(activeDevice.serial);
            showNotice(`Switched to Wi-Fi at ${ep} — you can unplug USB now.`);
            await refreshAll();
          } catch (e) {
            showError(String(e));
          }
        }}
        onNotice={showNotice}
        onError={showError}
      />
    {:else if tab === 'display'}
      <DisplayGamingTab
        device={activeDevice}
        onNotice={showNotice}
        onError={showError}
      />
    {:else if tab === 'files' || tab === 'system' || tab === 'console'}
      <PowerTabs
        {tab}
        device={activeDevice}
        recordingActive={snapshot?.controls.recording_active ?? false}
        onRecordToggle={() => void toggleRecord()}
        onNotice={showNotice}
        onError={showError}
      />
    {:else}
      <SubTabs
        {tab}
        device={activeDevice}
        {snapshot}
        {tools}
        bind:mirrorOpts
        onNotice={showNotice}
        onError={showError}
        onInstallTools={() => void installTools()}
        onOpenPair={() => (modal = 'pair')}
        onUnmuteApp={(pkg) => void unmuteApp(pkg)}
        onMirrorApp={(pkg, size) => void runMirror(pkg, size)}
        onScreenshot={() => void takeScreenshot()}
      />
    {/if}
  </div>
</div>

<Modals
  {modal}
  {devices}
  soundDisplay={snapshot?.sound_display ?? null}
  {timeline}
  {captured}
  commands={COMMANDS}
  onClose={() => (modal = 'none')}
  onPairWifi={async (addr, code) => {
    const out = await mobileIpc.pairWifi(addr, code);
    await refreshAll();
    return out;
  }}
  onConnectWifi={async (addr) => {
    const out = await mobileIpc.connectWifi(addr);
    await refreshAll();
    return out;
  }}
  onEnableWifi={async (s) => {
    const ep = await mobileIpc.enableWifi(s);
    await refreshAll();
    return ep;
  }}
  onWifiConnected={(msg) => {
    if (msg) showNotice(msg);
    void refreshAll();
  }}
  onApplySoundDisplay={applySoundDisplay}
  onClearTimeline={() => {
    void mobileIpc.clearTimeline();
    timeline = [];
  }}
  onRunCommand={runCommand}
  onOpenPath={(p) => void openPath(p).catch(() => {})}
/>

<style>
  .mob-shell {
    display: grid;
    grid-template-columns: 70px minmax(0, 1fr);
    gap: 16px;
    align-items: start;
    min-height: calc(100vh - 110px);
  }
  @media (max-width: 720px) {
    .mob-shell {
      grid-template-columns: 1fr;
    }
  }

  /* Vertical mini rail */
  .mob-rail {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 10px 6px;
    border-radius: var(--radius-l);
    background: var(--surface-1);
    border: 1px solid var(--border);
  }
  .rail-brand {
    width: 38px;
    height: 38px;
    border-radius: 12px;
    background: var(--accent);
    color: var(--on-accent);
    display: grid;
    place-items: center;
    margin-bottom: 4px;
  }
  .rail-tabs {
    display: grid;
    gap: 4px;
    width: 100%;
  }
  .rail-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 6px 2px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--text-3);
    cursor: pointer;
    transition: background var(--dur-1), color var(--dur-1);
  }
  .rail-btn:hover {
    color: var(--text);
  }
  .r-ico {
    width: 36px;
    height: 26px;
    border-radius: var(--radius-pill);
    display: grid;
    place-items: center;
    transition: background var(--dur-1);
  }
  .rail-btn.on {
    color: var(--text);
  }
  .rail-btn.on .r-ico {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .r-lbl {
    font-size: 10.5px;
    font-weight: 600;
    line-height: 1;
  }
  .rail-add {
    margin-top: 6px;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    color: var(--text-2);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .rail-add:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  /* Main column */
  .mob-main {
    display: grid;
    gap: 12px;
    min-width: 0;
  }
  .mob-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    padding-bottom: 4px;
  }
  .tb-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .mob-topbar h1 {
    font-size: 21px;
    margin: 0;
  }
  .dev-pick {
    padding: 5px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border-strong);
    background: var(--surface-1);
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
  }
  .tb-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tb-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 11px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-1);
    color: var(--text-2);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .tb-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .tb-ico {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-1);
    color: var(--text-2);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .tb-ico:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .tb-ico.spin :global(svg) {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Setup & feedback banners */
  .setup-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    padding: 13px 16px;
    border-radius: var(--radius-m);
    background: var(--surface-1);
    border: 1px solid var(--accent);
  }
  .setup-banner p {
    margin: 2px 0 0;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .prog-wrap {
    margin-top: 8px;
    max-width: 320px;
  }
  .primary {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 15px;
    border: 0;
    border-radius: var(--radius-s);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 12px;
    border-radius: var(--radius-s);
    font-size: 12.5px;
    font-weight: 600;
  }
  .banner.ok {
    background: var(--surface-1);
    border: 1px solid var(--ok);
    color: var(--ok);
  }
  .banner.err {
    background: var(--surface-1);
    border: 1px solid var(--danger);
    color: var(--danger);
  }
  .xbtn {
    border: 0;
    background: none;
    color: inherit;
    display: grid;
    cursor: pointer;
  }

  @media (max-width: 720px) {
    .mob-rail {
      position: static;
      flex-direction: row;
      overflow-x: auto;
      padding: 8px;
    }
    .rail-tabs {
      display: flex;
      gap: 4px;
    }
    .rail-add {
      margin-top: 0;
      margin-left: auto;
      flex: none;
    }
  }
</style>
