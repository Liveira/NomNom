<script lang="ts">
  import { untrack } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { openPath } from '@tauri-apps/plugin-opener';
  import { formatBytes } from '../../lib/format';
  import Icon from '../../lib/icons/Icon.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import {
    mobileIpc,
    type MobileDevice,
    type MobileFsEntry,
    type MobileFsListing,
    type MobilePortTunnel,
    type MobileSystemInfo,
  } from '../../lib/ipc';
  import type { MobileTab } from './logic';

  let {
    tab,
    device,
    recordingActive,
    onRecordToggle,
    onNotice,
    onError,
  }: {
    tab: MobileTab;
    device: MobileDevice | null;
    recordingActive: boolean;
    onRecordToggle: () => void;
    onNotice: (msg: string) => void;
    onError: (msg: string) => void;
  } = $props();

  const serial = $derived(device?.state === 'device' ? device.serial : null);

  // ---- Files state ----
  let currentDir = $state('/sdcard');
  let listing = $state<MobileFsListing | null>(null);
  let fsLoading = $state(false);
  let fsQuery = $state('');
  let makingDir = $state(false);
  let newDirName = $state('');
  let renaming = $state<MobileFsEntry | null>(null);
  let renameVal = $state('');
  let confirmDel = $state<string | null>(null);

  const SHORTCUTS = [
    { label: 'Internal', dir: '/sdcard' },
    { label: 'Download', dir: '/sdcard/Download' },
    { label: 'DCIM', dir: '/sdcard/DCIM' },
    { label: 'Pictures', dir: '/sdcard/Pictures' },
    { label: 'Music', dir: '/sdcard/Music' },
    { label: 'Documents', dir: '/sdcard/Documents' },
  ];

  const breadcrumbs = $derived(() => {
    const parts = currentDir.replace(/^\/sdcard\/?/, '').split('/').filter(Boolean);
    const out = [{ label: 'Internal Storage', dir: '/sdcard' }];
    let acc = '/sdcard';
    for (const p of parts) {
      acc += `/${p}`;
      out.push({ label: p, dir: acc });
    }
    return out;
  });

  const filteredEntries = $derived(
    (listing?.entries ?? []).filter((e) => {
      const q = fsQuery.trim().toLowerCase();
      return !q || e.name.toLowerCase().includes(q);
    }),
  );

  // ---- System state ----
  let sysInfo = $state<MobileSystemInfo | null>(null);
  let sysLoading = $state(false);
  let dpiInput = $state('');
  let sizeInput = $state('');
  let confirmPower = $state<'reboot' | 'recovery' | 'bootloader' | null>(null);

  const ramUsedKb = $derived(sysInfo ? Math.max(0, sysInfo.ram_total_kb - sysInfo.ram_avail_kb) : 0);
  const ramFrac = $derived(sysInfo && sysInfo.ram_total_kb > 0 ? ramUsedKb / sysInfo.ram_total_kb : 0);
  const storFrac = $derived(
    sysInfo && sysInfo.storage_total_kb > 0 ? sysInfo.storage_used_kb / sysInfo.storage_total_kb : 0,
  );
  const fsStorFrac = $derived(
    listing && listing.storage_total_kb > 0 ? listing.storage_used_kb / listing.storage_total_kb : 0,
  );

  // ---- Console state ----
  let consoleSub = $state<'shell' | 'logcat' | 'tunnels'>('shell');
  let shellInput = $state('');
  let shellBusy = $state(false);
  let shellHistory = $state<string[]>([]);
  let histIdx = $state(-1);
  let shellLog = $state<{ cmd: string; out: string; err: string; ok: boolean }[]>([]);

  let logLevel = $state<'V' | 'I' | 'W' | 'E'>('W');
  let logFilter = $state('');
  let logText = $state('');
  let logLoading = $state(false);

  let tunnels = $state<MobilePortTunnel[]>([]);
  let tunnelDir = $state<'reverse' | 'forward'>('reverse');
  let localPort = $state(5173);
  let remotePort = $state(5173);

  const filteredLogLines = $derived(() => {
    const q = logFilter.trim().toLowerCase();
    if (!logText) return '';
    if (!q) return logText;
    return logText
      .split('\n')
      .filter((l) => l.toLowerCase().includes(q))
      .join('\n');
  });

  async function loadDir(dir = currentDir) {
    if (!serial) return;
    fsLoading = true;
    try {
      listing = await mobileIpc.fsList(serial, dir);
      currentDir = listing.dir;
    } catch (e) {
      onError(String(e));
    } finally {
      fsLoading = false;
    }
  }

  async function uploadHere() {
    if (!serial) return;
    const picked = await open({ multiple: true });
    if (!picked) return;
    try {
      const files = Array.isArray(picked) ? picked : [picked];
      const n = await mobileIpc.fsPush(serial, currentDir, files);
      onNotice(`Uploaded ${n} ${n === 1 ? 'file' : 'files'} to ${currentDir}`);
      await loadDir(currentDir);
    } catch (e) {
      onError(String(e));
    }
  }

  async function pullEntry(e: MobileFsEntry) {
    if (!serial) return;
    try {
      const saved = await mobileIpc.fsPull(serial, e.path);
      onNotice(`Saved ${e.name}`);
      void openPath(saved).catch(() => {});
    } catch (err) {
      onError(String(err));
    }
  }

  async function createFolder() {
    if (!serial || !newDirName.trim()) return;
    try {
      await mobileIpc.fsMkdir(serial, `${currentDir}/${newDirName.trim()}`);
      newDirName = '';
      makingDir = false;
      await loadDir(currentDir);
    } catch (e) {
      onError(String(e));
    }
  }

  async function submitRename() {
    if (!serial || !renaming || !renameVal.trim()) return;
    const parent = renaming.path.slice(0, renaming.path.lastIndexOf('/')) || '/sdcard';
    try {
      await mobileIpc.fsRename(serial, renaming.path, `${parent}/${renameVal.trim()}`);
      renaming = null;
      await loadDir(currentDir);
    } catch (e) {
      onError(String(e));
    }
  }

  async function deleteEntry(path: string) {
    if (!serial) return;
    try {
      await mobileIpc.fsDelete(serial, path);
      confirmDel = null;
      await loadDir(currentDir);
    } catch (e) {
      onError(String(e));
    }
  }

  async function loadSystem() {
    if (!serial) return;
    sysLoading = true;
    try {
      sysInfo = await mobileIpc.systemInfo(serial);
    } catch (e) {
      onError(String(e));
    } finally {
      sysLoading = false;
    }
  }

  async function applyTweak(
    tweak: 'dark_mode' | 'anim_scale' | 'density' | 'size' | 'show_touches' | 'stay_awake' | 'airplane',
    val: string,
  ) {
    if (!serial) return;
    try {
      onNotice(await mobileIpc.setTweak(serial, tweak, val));
      await loadSystem();
    } catch (e) {
      onError(String(e));
    }
  }

  async function runPower(mode: 'wake' | 'sleep' | 'reboot' | 'recovery' | 'bootloader') {
    if (!serial) return;
    confirmPower = null;
    try {
      onNotice(await mobileIpc.power(serial, mode));
    } catch (e) {
      onError(String(e));
    }
  }

  async function runShell(cmd = shellInput) {
    if (!serial || !cmd.trim()) return;
    const clean = cmd.trim();
    shellBusy = true;
    shellInput = '';
    shellHistory = [clean, ...shellHistory.filter((c) => c !== clean)].slice(0, 40);
    histIdx = -1;
    try {
      const res = await mobileIpc.shell(serial, clean);
      shellLog = [
        ...shellLog.slice(-24),
        { cmd: clean, out: res.stdout.trimEnd(), err: res.stderr.trimEnd(), ok: res.ok },
      ];
    } catch (e) {
      shellLog = [...shellLog.slice(-24), { cmd: clean, out: '', err: String(e), ok: false }];
    } finally {
      shellBusy = false;
    }
  }

  function onShellKey(e: KeyboardEvent) {
    if (e.key === 'ArrowUp' && shellHistory.length > 0) {
      e.preventDefault();
      histIdx = Math.min(shellHistory.length - 1, histIdx + 1);
      shellInput = shellHistory[histIdx];
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (histIdx <= 0) {
        histIdx = -1;
        shellInput = '';
      } else {
        histIdx -= 1;
        shellInput = shellHistory[histIdx];
      }
    }
  }

  async function loadLogcat() {
    if (!serial) return;
    logLoading = true;
    try {
      logText = await mobileIpc.logcat(serial, logLevel, 200);
    } catch (e) {
      onError(String(e));
    } finally {
      logLoading = false;
    }
  }

  async function loadTunnels() {
    if (!serial) return;
    tunnels = await mobileIpc.tunnelsList(serial).catch(() => []);
  }

  async function addTunnel(dir = tunnelDir, lp = localPort, rp = remotePort) {
    if (!serial) return;
    try {
      tunnels = await mobileIpc.tunnelAdd(serial, dir, lp, rp);
      onNotice(`Added ${dir} tunnel :${lp} ↔ :${rp}`);
    } catch (e) {
      onError(String(e));
    }
  }

  async function removeTunnel(t: MobilePortTunnel) {
    if (!serial) return;
    try {
      tunnels = await mobileIpc.tunnelRemove(
        serial,
        t.direction === 'forward' ? 'forward' : 'reverse',
        t.local,
      );
    } catch (e) {
      onError(String(e));
    }
  }

  $effect(() => {
    void serial;
    if (tab === 'files' && serial) void loadDir(untrack(() => currentDir));
    else if (tab === 'system' && serial) void loadSystem();
    else if (tab === 'console' && serial) void loadTunnels();
  });
</script>

{#if tab === 'files'}
  <section class="panel-box">
    <div class="bar">
      <div>
        <h3>Phone Files</h3>
        <p class="sub">Browse internal storage, upload files into any folder, download, rename or organize.</p>
      </div>
      <div class="bar-acts">
        <button class="primary" disabled={!serial} onclick={uploadHere}>
          <Icon name="upload" size={14} /> Upload here
        </button>
        <button class="plain" disabled={!serial} onclick={() => (makingDir = !makingDir)}>
          <Icon name="folder-plus" size={14} /> New folder
        </button>
        <button class="plain" disabled={!serial || fsLoading} onclick={() => void loadDir(currentDir)}>
          <Icon name="refresh" size={14} />
        </button>
      </div>
    </div>

    {#if !serial}
      <p class="sub">Connect your Android phone to browse its files.</p>
    {:else}
      {#if listing && listing.storage_total_kb > 0}
        <div class="stor-strip">
          <div class="sl-head">
            <span>Internal storage</span>
            <span class="tabular">
              {formatBytes(listing.storage_avail_kb * 1024)} free of {formatBytes(listing.storage_total_kb * 1024)}
            </span>
          </div>
          <ProgressBar value={fsStorFrac} label="Phone internal storage usage" />
        </div>
      {/if}

      <div class="shortcut-row">
        {#each SHORTCUTS as sc (sc.dir)}
          <button
            class="pill-chip"
            class:on={currentDir === sc.dir}
            onclick={() => void loadDir(sc.dir)}
          >
            {sc.label}
          </button>
        {/each}
        <input
          class="fs-search"
          type="search"
          placeholder="Filter this folder…"
          bind:value={fsQuery}
          aria-label="Filter files"
        />
      </div>

      <nav class="crumbs" aria-label="Folder path">
        {#each breadcrumbs() as bc, i (bc.dir)}
          {#if i > 0}<span class="sep">/</span>{/if}
          <button class="crumb" onclick={() => void loadDir(bc.dir)}>{bc.label}</button>
        {/each}
      </nav>

      {#if makingDir}
        <form
          class="inline-form"
          onsubmit={(e) => {
            e.preventDefault();
            void createFolder();
          }}
        >
          <input type="text" placeholder="Folder name…" bind:value={newDirName} />
          <button type="submit" class="primary" disabled={!newDirName.trim()}>Create</button>
          <button type="button" class="plain" onclick={() => (makingDir = false)}>Cancel</button>
        </form>
      {/if}

      {#if renaming}
        <form
          class="inline-form"
          onsubmit={(e) => {
            e.preventDefault();
            void submitRename();
          }}
        >
          <span class="sub">Rename:</span>
          <input type="text" bind:value={renameVal} />
          <button type="submit" class="primary" disabled={!renameVal.trim()}>Save</button>
          <button type="button" class="plain" onclick={() => (renaming = null)}>Cancel</button>
        </form>
      {/if}

      {#if filteredEntries.length === 0}
        <p class="sub">{fsLoading ? 'Reading folder…' : 'This folder is empty.'}</p>
      {:else}
        <ul class="fs-list">
          {#each filteredEntries as item (item.path)}
            <li>
              <button
                class="fs-main"
                onclick={() => (item.is_dir ? void loadDir(item.path) : void pullEntry(item))}
              >
                <span class="fs-ico" class:dir={item.is_dir}>
                  <Icon name={item.is_dir ? 'folder' : 'file'} size={15} />
                </span>
                <span class="fs-name" translate="no" title={item.path}>{item.name}</span>
                <span class="fs-sz tabular">{item.is_dir ? 'Folder' : formatBytes(item.size)}</span>
              </button>
              <div class="fs-acts">
                {#if !item.is_dir}
                  <button class="plain sm" onclick={() => void pullEntry(item)}>Download</button>
                {/if}
                <button
                  class="icon-sm"
                  title="Rename"
                  aria-label="Rename"
                  onclick={() => {
                    renaming = item;
                    renameVal = item.name;
                  }}
                >
                  <Icon name="pencil" size={13} />
                </button>
                {#if confirmDel === item.path}
                  <button class="plain sm danger-btn" onclick={() => void deleteEntry(item.path)}>Delete</button>
                  <button class="plain sm" onclick={() => (confirmDel = null)}>Cancel</button>
                {:else}
                  <button
                    class="icon-sm"
                    title="Delete"
                    aria-label="Delete"
                    onclick={() => (confirmDel = item.path)}
                  >
                    <Icon name="trash" size={13} />
                  </button>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>

{:else if tab === 'system'}
  <div class="sys-grid">
    <section class="panel-box">
      <div class="bar">
        <h3>Device & Hardware</h3>
        <button class="plain" disabled={!serial || sysLoading} onclick={() => void loadSystem()}>
          <Icon name="refresh" size={14} /> Refresh
        </button>
      </div>

      {#if !serial}
        <p class="sub">Connect your Android phone to inspect hardware, memory, battery and tweaks.</p>
      {:else if !sysInfo}
        <p class="sub">Reading device telemetry…</p>
      {:else}
        <div class="meter-grid">
          <div class="meter">
            <div class="sl-head">
              <span>Memory (RAM)</span>
              <strong class="tabular">{Math.round(ramFrac * 100)}%</strong>
            </div>
            <ProgressBar value={ramFrac} label="Phone RAM usage" />
            <span class="sub tabular">
              {formatBytes(ramUsedKb * 1024)} of {formatBytes(sysInfo.ram_total_kb * 1024)}
            </span>
          </div>

          <div class="meter">
            <div class="sl-head">
              <span>Internal Storage</span>
              <strong class="tabular">{Math.round(storFrac * 100)}%</strong>
            </div>
            <ProgressBar value={storFrac} label="Phone storage usage" />
            <span class="sub tabular">
              {formatBytes(sysInfo.storage_avail_kb * 1024)} free of {formatBytes(sysInfo.storage_total_kb * 1024)}
            </span>
          </div>
        </div>

        <dl class="specs">
          <div><dt>Model</dt><dd translate="no">{sysInfo.manufacturer} {sysInfo.model}</dd></div>
          <div><dt>Android</dt><dd class="tabular">{sysInfo.android_version} (API {sysInfo.sdk_level})</dd></div>
          <div><dt>Security patch</dt><dd class="tabular">{sysInfo.security_patch || '—'}</dd></div>
          <div><dt>CPU / SoC</dt><dd translate="no">{sysInfo.hardware || '—'} · {sysInfo.cpu_cores} cores ({sysInfo.abi})</dd></div>
          <div><dt>Display</dt><dd class="tabular">{sysInfo.screen_size} · {sysInfo.screen_density}</dd></div>
          <div>
            <dt>Battery</dt>
            <dd class="tabular">
              {sysInfo.battery_level}% · {sysInfo.battery_temp_c} °C · {sysInfo.battery_voltage_mv} mV · {sysInfo.battery_health} ({sysInfo.battery_tech || 'Li-ion'})
            </dd>
          </div>
          <div><dt>Wi-Fi / IP</dt><dd translate="no">{sysInfo.wifi_ssid || '—'} · {sysInfo.wifi_ip || '—'}</dd></div>
          <div><dt>Carrier</dt><dd translate="no">{sysInfo.carrier || '—'}</dd></div>
          <div><dt>Kernel</dt><dd class="mono-sm" translate="no">{sysInfo.kernel || '—'}</dd></div>
        </dl>
      {/if}
    </section>

    <div class="sys-side">
      <section class="panel-box">
        <h3>Live Android Tweaks</h3>
        <div class="tweak-row">
          <div>
            <strong>Dark mode</strong>
            <p class="sub">Switch Android system theme immediately.</p>
          </div>
          <Switch
            checked={sysInfo?.dark_mode ?? false}
            disabled={!serial}
            label="Dark mode"
            onchange={(v) => void applyTweak('dark_mode', v ? 'on' : 'off')}
          />
        </div>

        <div class="tweak-row">
          <div>
            <strong>Animation speed</strong>
            <p class="sub">Set 0.5x or 0x to make Android feel snappier.</p>
          </div>
          <div class="seg-mini">
            {#each [['0.0', 'Off'], ['0.5', '0.5x'], ['1.0', '1x']] as [val, lbl] (val)}
              <button
                class:on={sysInfo?.anim_scale === val || (val === '0.0' && sysInfo?.anim_scale === '0')}
                disabled={!serial}
                onclick={() => void applyTweak('anim_scale', val)}
              >
                {lbl}
              </button>
            {/each}
          </div>
        </div>

        <div class="tweak-row">
          <div>
            <strong>Show screen touches</strong>
            <p class="sub">Helpful when recording tutorials or demos.</p>
          </div>
          <Switch
            checked={sysInfo?.show_touches ?? false}
            disabled={!serial}
            label="Show screen touches"
            onchange={(v) => void applyTweak('show_touches', v ? 'on' : 'off')}
          />
        </div>

        <div class="tweak-row">
          <div>
            <strong>Stay awake while charging</strong>
            <p class="sub">Keep the screen on whenever USB is plugged in.</p>
          </div>
          <Switch
            checked={sysInfo?.stay_awake_plugged ?? false}
            disabled={!serial}
            label="Stay awake while charging"
            onchange={(v) => void applyTweak('stay_awake', v ? 'on' : 'off')}
          />
        </div>

        <div class="tweak-row">
          <div>
            <strong>Airplane mode</strong>
            <p class="sub">Toggle cellular and wireless radios.</p>
          </div>
          <Switch
            checked={sysInfo?.airplane_mode ?? false}
            disabled={!serial}
            label="Airplane mode"
            onchange={(v) => void applyTweak('airplane', v ? 'on' : 'off')}
          />
        </div>

        <hr />
        <h3>Display scaling (DPI & Resolution)</h3>
        <div class="inline-form">
          <input type="text" placeholder="DPI (e.g. 420)" bind:value={dpiInput} />
          <button class="plain" disabled={!serial || !dpiInput.trim()} onclick={() => void applyTweak('density', dpiInput.trim())}>
            Set DPI
          </button>
          <button class="plain" disabled={!serial} onclick={() => void applyTweak('density', 'reset')}>
            Reset
          </button>
        </div>
        <div class="inline-form">
          <input type="text" placeholder="Size (e.g. 1080x2340)" bind:value={sizeInput} />
          <button class="plain" disabled={!serial || !sizeInput.trim()} onclick={() => void applyTweak('size', sizeInput.trim())}>
            Set size
          </button>
          <button class="plain" disabled={!serial} onclick={() => void applyTweak('size', 'reset')}>
            Reset
          </button>
        </div>
      </section>

      <section class="panel-box">
        <h3>Screen Recorder & Power</h3>
        <div class="power-grid">
          <button
            class="primary"
            class:rec-on={recordingActive}
            disabled={!serial}
            onclick={onRecordToggle}
          >
            <Icon name="record" size={14} />
            {recordingActive ? 'Stop recording' : 'Record screen (MKV)'}
          </button>
          <button class="plain" disabled={!serial} onclick={() => void runPower('wake')}>
            <Icon name="sun" size={14} /> Wake
          </button>
          <button class="plain" disabled={!serial} onclick={() => void runPower('sleep')}>
            <Icon name="lock" size={14} /> Lock
          </button>
          <button class="plain" disabled={!serial} onclick={() => (confirmPower = 'reboot')}>
            <Icon name="refresh" size={14} /> Reboot
          </button>
          <button class="plain" disabled={!serial} onclick={() => (confirmPower = 'recovery')}>
            Recovery
          </button>
          <button class="plain" disabled={!serial} onclick={() => (confirmPower = 'bootloader')}>
            Bootloader
          </button>
        </div>
        {#if confirmPower}
          <div class="confirm-bar">
            <span>Reboot phone ({confirmPower}) now?</span>
            <button class="plain danger-btn" onclick={() => confirmPower && void runPower(confirmPower)}>
              Yes, reboot
            </button>
            <button class="plain" onclick={() => (confirmPower = null)}>Cancel</button>
          </div>
        {/if}
      </section>
    </div>
  </div>

{:else if tab === 'console'}
  <section class="panel-box">
    <div class="bar">
      <div class="seg-mini">
        <button class:on={consoleSub === 'shell'} onclick={() => (consoleSub = 'shell')}>ADB Shell</button>
        <button
          class:on={consoleSub === 'logcat'}
          onclick={() => {
            consoleSub = 'logcat';
            if (!logText && serial) void loadLogcat();
          }}
        >
          Logcat
        </button>
        <button
          class:on={consoleSub === 'tunnels'}
          onclick={() => {
            consoleSub = 'tunnels';
            if (serial) void loadTunnels();
          }}
        >
          Port Tunnels
        </button>
      </div>
      {#if consoleSub === 'shell' && shellLog.length > 0}
        <button class="plain sm" onclick={() => (shellLog = [])}>Clear output</button>
      {/if}
    </div>

    {#if !serial}
      <p class="sub">Connect your Android phone to use ADB Shell, Logcat and Port Tunnels.</p>
    {:else if consoleSub === 'shell'}
      <div class="shortcut-row">
        {#each ['getprop ro.product.model', 'wm size && wm density', 'ip route', 'df -h /data', 'dumpsys battery', 'ps -A | head -n 30'] as preset (preset)}
          <button class="pill-chip" disabled={shellBusy} onclick={() => void runShell(preset)}>
            {preset}
          </button>
        {/each}
      </div>

      <div class="term-out" role="log" aria-live="polite">
        {#if shellLog.length === 0}
          <span class="sub">Type any Android shell command below and press Enter. Use Up/Down arrows for history.</span>
        {:else}
          {#each shellLog as entry, idx (idx)}
            <div class="term-entry">
              <div class="term-cmd">$ {entry.cmd}</div>
              {#if entry.out}<pre>{entry.out}</pre>{/if}
              {#if entry.err}<pre class="term-err">{entry.err}</pre>{/if}
            </div>
          {/each}
        {/if}
      </div>

      <form
        class="inline-form"
        onsubmit={(e) => {
          e.preventDefault();
          void runShell();
        }}
      >
        <input
          type="text"
          class="mono-input"
          placeholder="adb shell command…"
          bind:value={shellInput}
          onkeydown={onShellKey}
          aria-label="ADB shell command"
        />
        <button type="submit" class="primary" disabled={shellBusy || !shellInput.trim()}>
          {shellBusy ? 'Running…' : 'Run'}
        </button>
      </form>

    {:else if consoleSub === 'logcat'}
      <div class="bar">
        <div class="bar-acts">
          <select bind:value={logLevel} onchange={() => void loadLogcat()} aria-label="Log level">
            <option value="V">Verbose (All)</option>
            <option value="I">Info+</option>
            <option value="W">Warnings+</option>
            <option value="E">Errors only</option>
          </select>
          <input type="search" placeholder="Filter tag or message…" bind:value={logFilter} />
        </div>
        <div class="bar-acts">
          <button class="plain" disabled={logLoading} onclick={() => void loadLogcat()}>
            <Icon name="refresh" size={14} /> {logLoading ? 'Loading…' : 'Fetch logs'}
          </button>
          <button
            class="plain"
            disabled={!logText}
            onclick={() => {
              void navigator.clipboard.writeText(filteredLogLines()).catch(() => {});
              onNotice('Copied logcat output');
            }}
          >
            <Icon name="copy" size={14} /> Copy
          </button>
          <button
            class="plain"
            onclick={async () => {
              if (!serial) return;
              await mobileIpc.logcatClear(serial);
              logText = '';
              onNotice('Cleared phone logcat buffer');
            }}
          >
            Clear buffer
          </button>
        </div>
      </div>
      <pre class="term-out logcat-pre">{filteredLogLines() || 'Click Fetch logs to read recent device logs.'}</pre>

    {:else if consoleSub === 'tunnels'}
      <p class="sub">
        <strong>Reverse</strong> lets your phone open a dev server running on this computer (for example
        <code>localhost:5173</code>). <strong>Forward</strong> lets this computer open a port running on the phone.
      </p>
      <div class="shortcut-row">
        {#each [3000, 5173, 8000, 8080] as p (p)}
          <button class="pill-chip" onclick={() => void addTunnel('reverse', p, p)}>
            Reverse localhost:{p}
          </button>
        {/each}
      </div>
      <form
        class="inline-form"
        onsubmit={(e) => {
          e.preventDefault();
          void addTunnel();
        }}
      >
        <select bind:value={tunnelDir} aria-label="Direction">
          <option value="reverse">Reverse (PC → Phone)</option>
          <option value="forward">Forward (Phone → PC)</option>
        </select>
        <input type="number" min="1" max="65535" bind:value={localPort} aria-label="PC port" />
        <span class="sub">↔</span>
        <input type="number" min="1" max="65535" bind:value={remotePort} aria-label="Phone port" />
        <button type="submit" class="primary">Add tunnel</button>
      </form>

      {#if tunnels.length === 0}
        <p class="sub">No active port tunnels.</p>
      {:else}
        <ul class="fs-list">
          {#each tunnels as t, idx (idx)}
            <li>
              <div class="fs-main static">
                <span class="pill-chip on">{t.direction}</span>
                <span class="mono-sm" translate="no">{t.local} ↔ {t.remote}</span>
              </div>
              <button class="plain sm" onclick={() => void removeTunnel(t)}>Remove</button>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .panel-box {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    padding: 18px 20px;
    box-shadow: var(--shadow-card);
    display: grid;
    gap: 12px;
    align-content: start;
  }
  .sys-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(300px, 1fr);
    gap: 14px;
    align-items: start;
  }
  @media (max-width: 900px) {
    .sys-grid { grid-template-columns: 1fr; }
  }
  .sys-side { display: grid; gap: 14px; }
  .bar { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .bar-acts { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  h3 { margin: 0; font-size: 15px; }
  .sub { margin: 2px 0 0; font-size: 12.5px; color: var(--text-3); }
  hr { border: 0; border-top: 1px solid var(--border); margin: 4px 0; }
  input[type='text'], input[type='search'], input[type='number'], select {
    padding: 7px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: var(--text); font-size: 13px;
  }
  input[type='number'] { width: 88px; }
  .mono-input { flex: 1; font-family: var(--font-mono); }
  .mono-sm { font-family: var(--font-mono); font-size: 12px; }
  .plain, .primary {
    display: inline-flex; align-items: center; gap: 6px; padding: 7px 13px;
    border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: inherit; font-weight: 600; font-size: 12.5px; cursor: pointer;
  }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .primary:hover:not(:disabled) { background: var(--accent-hover); }
  .primary.rec-on { background: var(--danger); color: #fff; }
  .plain.sm { padding: 4px 9px; font-size: 12px; }
  .danger-btn { color: var(--danger); }
  button:disabled { opacity: 0.45; cursor: default; }
  .icon-sm { border: 0; background: none; color: var(--text-3); padding: 4px; border-radius: 6px; display: grid; cursor: pointer; }
  .icon-sm:hover { background: var(--surface-3); color: var(--text); }

  /* Files */
  .stor-strip { display: grid; gap: 6px; padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .sl-head { display: flex; justify-content: space-between; font-size: 12.5px; }
  .shortcut-row { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .pill-chip { padding: 4px 10px; border-radius: var(--radius-s); border: 1px solid var(--border); background: var(--surface-2); color: var(--text-2); font-size: 12px; font-weight: 600; cursor: pointer; }
  .pill-chip:hover { background: var(--surface-3); color: var(--text); }
  .pill-chip.on { background: var(--accent-soft); border-color: var(--accent); color: var(--accent-text); }
  .fs-search { margin-left: auto; width: 190px; }
  .crumbs { display: flex; align-items: center; gap: 4px; flex-wrap: wrap; font-size: 12.5px; color: var(--text-3); }
  .crumb { border: 0; background: none; color: var(--text-2); font-weight: 600; cursor: pointer; padding: 2px 4px; border-radius: 5px; }
  .crumb:hover { background: var(--surface-2); color: var(--text); }
  .sep { opacity: 0.5; }
  .inline-form { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .inline-form input[type='text'] { flex: 1; min-width: 140px; }
  .fs-list { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; max-height: 440px; overflow: auto; }
  .fs-list li { display: flex; align-items: center; gap: 8px; padding: 4px 8px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .fs-main { flex: 1; min-width: 0; display: flex; align-items: center; gap: 9px; border: 0; background: none; color: inherit; text-align: left; padding: 4px 2px; cursor: pointer; }
  .fs-main.static { cursor: default; }
  .fs-ico { color: var(--text-3); display: grid; }
  .fs-ico.dir { color: var(--accent-text); }
  .fs-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; font-weight: 500; }
  .fs-sz { font-size: 12px; color: var(--text-3); flex: none; }
  .fs-acts { display: flex; align-items: center; gap: 4px; flex: none; }

  /* System */
  .meter-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .meter { display: grid; gap: 5px; padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .specs { margin: 0; display: grid; gap: 6px; }
  .specs > div { display: flex; justify-content: space-between; gap: 14px; padding: 6px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .specs > div:last-child { border-bottom: 0; }
  dt { color: var(--text-3); flex: none; }
  dd { margin: 0; font-weight: 600; text-align: right; overflow: hidden; text-overflow: ellipsis; }
  .tweak-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 7px 0; border-top: 1px solid var(--border); }
  .tweak-row:first-of-type { border-top: 0; }
  .seg-mini { display: inline-flex; padding: 2px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); gap: 2px; }
  .seg-mini button { border: 0; background: transparent; color: var(--text-2); padding: 4px 10px; border-radius: var(--radius-xs); font-size: 12px; font-weight: 600; cursor: pointer; }
  .seg-mini button.on { background: var(--accent); color: var(--on-accent); }
  .power-grid { display: flex; flex-wrap: wrap; gap: 8px; }
  .confirm-bar { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--danger); font-size: 12.5px; }

  /* Console */
  .term-out {
    min-height: 260px; max-height: 400px; overflow: auto; padding: 12px;
    border-radius: var(--radius-s); background: var(--bg); border: 1px solid var(--border);
    font-family: var(--font-mono); font-size: 12px; line-height: 1.45; user-select: text;
    display: grid; gap: 10px; align-content: start;
  }
  .term-entry { display: grid; gap: 3px; }
  .term-cmd { color: var(--accent-text); font-weight: 700; }
  .term-out pre { margin: 0; white-space: pre-wrap; word-break: break-word; color: var(--text-2); font: inherit; }
  .term-out .term-err { color: var(--danger); }
  .logcat-pre { display: block; white-space: pre-wrap; word-break: break-word; }
</style>
