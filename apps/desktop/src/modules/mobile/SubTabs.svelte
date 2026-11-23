<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { openPath } from '@tauri-apps/plugin-opener';
  import { app } from '../../app/state.svelte';
  import { formatBytes } from '../../lib/format';
  import Icon from '../../lib/icons/Icon.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import {
    mobileIpc,
    type MobileApp,
    type MobileCall,
    type MobileContact,
    type MobileDeckItem,
    type MobileDevice,
    type MobileMirrorOptions,
    type MobilePhoto,
    type MobileSms,
    type MobileSnapshot,
    type MobileToolsStatus,
  } from '../../lib/ipc';
  import {
    appBadgeColor,
    formatCallDuration,
    groupSmsThreads,
    relativeMinutes,
    type MobileTab,
  } from './logic';

  let {
    tab,
    device,
    snapshot,
    tools,
    mirrorOpts = $bindable(),
    onNotice,
    onError,
    onInstallTools,
    onOpenPair,
    onUnmuteApp,
    onMirrorApp,
    onScreenshot,
  }: {
    tab: MobileTab;
    device: MobileDevice | null;
    snapshot: MobileSnapshot | null;
    tools: MobileToolsStatus | null;
    mirrorOpts: MobileMirrorOptions;
    onNotice: (msg: string) => void;
    onError: (msg: string) => void;
    onInstallTools: () => void;
    onOpenPair: () => void;
    onUnmuteApp: (pkg: string) => void;
    onMirrorApp: (pkg: string, displaySize?: string) => void;
    onScreenshot: () => void;
  } = $props();

  const serial = $derived(device?.state === 'device' ? device.serial : null);

  // Messages state
  let smsList = $state<MobileSms[]>([]);
  let smsLoading = $state(false);
  let smsNote = $state('');
  let activeThread = $state<string | null>(null);
  let composeTo = $state('');
  let composeBody = $state('');
  const threads = $derived(groupSmsThreads(smsList));
  const selectedThread = $derived(
    threads.find((t) => t.address === activeThread) ?? threads[0] ?? null,
  );

  // Calls & Contacts state
  let calls = $state<MobileCall[]>([]);
  let callsLoading = $state(false);
  let callsNote = $state('');
  let dialNumber = $state('');
  let contacts = $state<MobileContact[]>([]);
  let contactQuery = $state('');
  let contactsNote = $state('');
  const filteredContacts = $derived(
    contacts.filter((c) => {
      const q = contactQuery.trim().toLowerCase();
      return !q || c.name.toLowerCase().includes(q) || c.number.includes(q);
    }),
  );

  // Photos state
  let photos = $state<MobilePhoto[]>([]);
  let photosLoading = $state(false);
  let photoFolder = $state('All');
  let previews = $state<Record<string, string>>({});
  let loadingPreview = $state<Record<string, boolean>>({});
  let lightbox = $state<MobilePhoto | null>(null);

  const photoFolders = $derived([
    'All',
    ...new Set(photos.map((p) => p.folder)),
  ]);
  const filteredPhotos = $derived(
    photoFolder === 'All' ? photos : photos.filter((p) => p.folder === photoFolder),
  );

  // Apps state
  let apps = $state<MobileApp[]>([]);
  let appsLoading = $state(false);
  let appQuery = $state('');
  let appFilter = $state<'user' | 'system' | 'disabled' | 'all'>('user');
  let appDisplaySize = $state<'880x1920' | '1600x1000' | '1080x1080'>('880x1920');
  let pullingAllPhotos = $state(false);
  const filteredApps = $derived(
    apps.filter((a) => {
      const q = appQuery.trim().toLowerCase();
      return !q || a.name.toLowerCase().includes(q) || a.package.toLowerCase().includes(q);
    }),
  );

  // Deck state
  let deck = $state<MobileDeckItem[]>([]);
  let addingDeck = $state(false);
  let newDeckTitle = $state('');
  let newDeckKind = $state<'key' | 'url' | 'app' | 'shell' | 'screenshot'>('key');
  let newDeckPayload = $state('play_pause');

  async function loadMessages() {
    if (!serial) return;
    smsLoading = true;
    smsNote = '';
    try {
      smsList = await mobileIpc.messages(serial);
    } catch (e) {
      smsNote = String(e);
    } finally {
      smsLoading = false;
    }
  }

  async function loadCalls() {
    if (!serial) return;
    callsLoading = true;
    callsNote = '';
    contactsNote = '';
    try {
      calls = await mobileIpc.calls(serial);
    } catch (e) {
      callsNote = String(e);
    } finally {
      callsLoading = false;
    }
    try {
      contacts = await mobileIpc.contacts(serial);
    } catch (e) {
      contactsNote = String(e);
    }
  }

  async function exportContacts() {
    if (!serial) return;
    try {
      const saved = await mobileIpc.exportContacts(serial);
      onNotice(`Exported contacts to ${saved}`);
      void openPath(saved).catch(() => {});
    } catch (e) {
      onError(String(e));
    }
  }

  async function loadPhotos() {
    if (!serial) return;
    photosLoading = true;
    try {
      photos = await mobileIpc.photos(serial, 48);
      // Preload first 8 thumbnails smoothly
      for (const p of photos.slice(0, 8)) {
        void fetchPreview(p.path);
      }
    } catch (e) {
      onError(String(e));
    } finally {
      photosLoading = false;
    }
  }

  async function fetchPreview(remotePath: string) {
    if (!serial || previews[remotePath] || loadingPreview[remotePath]) return;
    loadingPreview = { ...loadingPreview, [remotePath]: true };
    try {
      const url = await mobileIpc.photoPreview(serial, remotePath);
      previews = { ...previews, [remotePath]: url };
    } catch {
      /* skip oversized or unreadable */
    } finally {
      const next = { ...loadingPreview };
      delete next[remotePath];
      loadingPreview = next;
    }
  }

  async function loadApps() {
    if (!serial) return;
    appsLoading = true;
    try {
      apps = await mobileIpc.apps(serial, appFilter);
    } catch (e) {
      onError(String(e));
    } finally {
      appsLoading = false;
    }
  }

  async function pullAllShownPhotos() {
    if (!serial || filteredPhotos.length === 0) return;
    pullingAllPhotos = true;
    try {
      const n = await mobileIpc.pullPhotos(
        serial,
        filteredPhotos.map((p) => p.path),
      );
      onNotice(`Backed up ${n} ${n === 1 ? 'photo' : 'photos'} to From phone folder`);
    } catch (e) {
      onError(String(e));
    } finally {
      pullingAllPhotos = false;
    }
  }

  async function runAppAction(
    a: MobileApp,
    action: 'force_stop' | 'clear_data' | 'disable' | 'enable' | 'app_info' | 'pull_apk',
  ) {
    if (!serial) return;
    try {
      onNotice(await mobileIpc.appAction(serial, a.package, action));
      if (action === 'disable' || action === 'enable') await loadApps();
    } catch (e) {
      onError(String(e));
    }
  }

  async function loadDeck() {
    deck = await mobileIpc.deckList().catch(() => []);
  }

  $effect(() => {
    void serial;
    if (tab === 'messages' && serial) void loadMessages();
    else if (tab === 'calls' && serial) void loadCalls();
    else if (tab === 'photos' && serial) void loadPhotos();
    else if (tab === 'apps' && serial) void loadApps();
    else if (tab === 'deck') void loadDeck();
  });

  async function sendSms(target: string) {
    if (!serial || !target.trim() || !composeBody.trim()) return;
    try {
      const msg = await mobileIpc.sendSms(serial, target, composeBody);
      composeBody = '';
      onNotice(msg);
    } catch (e) {
      onError(String(e));
    }
  }

  async function dial(num: string) {
    if (!serial || !num.trim()) return;
    try {
      onNotice(await mobileIpc.dial(serial, num));
    } catch (e) {
      onError(String(e));
    }
  }

  async function callKey(k: 'answer' | 'end_call' | 'mute') {
    if (!serial) return;
    try {
      await mobileIpc.keyevent(serial, k);
    } catch (e) {
      onError(String(e));
    }
  }

  async function pullPhoto(p: MobilePhoto) {
    if (!serial) return;
    try {
      const saved = await mobileIpc.pullPhoto(serial, p.path);
      onNotice(`Saved ${p.name}`);
      void openPath(saved).catch(() => {});
    } catch (e) {
      onError(String(e));
    }
  }

  async function removePhoto(p: MobilePhoto) {
    if (!serial) return;
    try {
      await mobileIpc.deletePhoto(serial, p.path);
      photos = photos.filter((x) => x.path !== p.path);
      if (lightbox?.path === p.path) lightbox = null;
      onNotice(`Deleted ${p.name} from phone`);
    } catch (e) {
      onError(String(e));
    }
  }

  async function installApk() {
    if (!serial) return;
    const picked = await open({
      multiple: false,
      filters: [{ name: 'Android package', extensions: ['apk'] }],
    });
    if (typeof picked !== 'string') return;
    try {
      onNotice(await mobileIpc.installApk(serial, picked));
      await loadApps();
    } catch (e) {
      onError(String(e));
    }
  }

  async function uninstallApp(a: MobileApp) {
    if (!serial) return;
    try {
      await mobileIpc.uninstallApp(serial, a.package);
      apps = apps.filter((x) => x.package !== a.package);
      onNotice(`Uninstalled ${a.name}`);
    } catch (e) {
      onError(String(e));
    }
  }

  async function runDeck(item: MobileDeckItem) {
    if (!serial) return;
    try {
      onNotice(await mobileIpc.deckRun(serial, item));
    } catch (e) {
      onError(String(e));
    }
  }

  async function saveDeck() {
    if (!newDeckTitle.trim()) return;
    try {
      deck = await mobileIpc.deckSave({
        id: '',
        title: newDeckTitle.trim(),
        icon:
          newDeckKind === 'url'
            ? 'link'
            : newDeckKind === 'app'
              ? 'grid'
              : newDeckKind === 'screenshot'
                ? 'image'
                : newDeckKind === 'shell'
                  ? 'terminal'
                  : 'zap',
        kind: newDeckKind,
        payload: newDeckPayload.trim(),
      });
      newDeckTitle = '';
      newDeckPayload = '';
      addingDeck = false;
    } catch (e) {
      onError(String(e));
    }
  }
</script>

{#if tab === 'messages'}
  <section class="panel-box">
    <div class="bar">
      <div>
        <h3>Messages</h3>
        <p class="sub">Read SMS threads and send messages directly from your computer.</p>
      </div>
      <div class="bar-acts">
        <button class="plain" disabled={!serial || smsLoading} onclick={loadMessages}>
          <Icon name="refresh" size={14} /> Refresh
        </button>
        <button
          class="primary"
          disabled={!serial}
          onclick={() => onMirrorApp('com.google.android.apps.messaging')}
        >
          <Icon name="cast" size={14} /> Open Messages window
        </button>
      </div>
    </div>

    {#if !serial}
      <p class="sub">Connect your Android phone to view and send messages.</p>
    {:else}
      {#if smsNote}<p class="warn">{smsNote}</p>{/if}

      <div class="msg-split">
        <div class="thread-list">
          <div class="new-sms">
            <input type="text" placeholder="Phone number…" bind:value={composeTo} />
          </div>
          {#if threads.length === 0 && !smsLoading}
            <p class="sub pad">No SMS threads loaded.</p>
          {:else}
            {#each threads as t (t.address)}
              <button
                class="thread-row"
                class:on={selectedThread?.address === t.address}
                onclick={() => {
                  activeThread = t.address;
                  composeTo = t.address;
                }}
              >
                <span class="avatar" style:background={appBadgeColor(t.address)}>
                  {t.address.slice(-2)}
                </span>
                <span class="t-meta">
                  <strong translate="no">{t.address}</strong>
                  <span class="t-snip" translate="no">{t.lastBody}</span>
                </span>
                <span class="t-when">{relativeMinutes(t.lastDateMs)}</span>
              </button>
            {/each}
          {/if}
        </div>

        <div class="chat-pane">
          <div class="chat-log">
            {#if selectedThread}
              {#each selectedThread.messages as m (m.id)}
                <div class="bubble" class:out={!m.incoming}>
                  <span translate="no">{m.body}</span>
                  <small>{relativeMinutes(m.date_ms)}</small>
                </div>
              {/each}
            {:else}
              <p class="sub">Select a thread or type a phone number to send a new SMS.</p>
            {/if}
          </div>
          <form
            class="chat-form"
            onsubmit={(e) => {
              e.preventDefault();
              void sendSms(composeTo || selectedThread?.address || '');
            }}
          >
            <input
              type="text"
              placeholder="Write a message…"
              bind:value={composeBody}
              aria-label="Message text"
            />
            <button
              type="submit"
              class="primary"
              disabled={!composeBody.trim() || !(composeTo.trim() || selectedThread?.address)}
            >
              <Icon name="send" size={14} /> Send
            </button>
          </form>
        </div>
      </div>
    {/if}
  </section>

{:else if tab === 'calls'}
  <div class="two-col">
    <section class="panel-box">
      <h3>Dialer & Call controls</h3>
      <p class="sub">Place a call on your phone or control an active call.</p>
      <div class="dial-display">
        <input
          type="text"
          placeholder="Enter phone number…"
          bind:value={dialNumber}
          aria-label="Phone number"
        />
        <button class="primary" disabled={!serial || !dialNumber.trim()} onclick={() => dial(dialNumber)}>
          <Icon name="phone" size={15} /> Call
        </button>
      </div>
      <div class="keypad">
        {#each ['1','2','3','4','5','6','7','8','9','*','0','#'] as k (k)}
          <button class="kcell" onclick={() => (dialNumber += k)}>{k}</button>
        {/each}
      </div>
      <div class="call-ctrls">
        <button class="plain ok-btn" disabled={!serial} onclick={() => callKey('answer')}>
          <Icon name="phone" size={14} /> Answer
        </button>
        <button class="plain danger-btn" disabled={!serial} onclick={() => callKey('end_call')}>
          <Icon name="phone-off" size={14} /> End call
        </button>
        <button class="plain" disabled={!serial} onclick={() => callKey('mute')}>
          <Icon name="volume" size={14} /> Mute
        </button>
      </div>
    </section>

    <section class="panel-box">
      <div class="bar">
        <h3>Recent calls</h3>
        <button class="plain" disabled={!serial || callsLoading} onclick={loadCalls}>
          <Icon name="refresh" size={14} /> Refresh
        </button>
      </div>
      {#if !serial}
        <p class="sub">Connect your Android phone to view recent calls.</p>
      {:else if callsNote}
        <p class="warn">{callsNote}</p>
      {:else if calls.length === 0}
        <p class="sub">{callsLoading ? 'Loading call log…' : 'No recent calls found.'}</p>
      {:else}
        <ul class="call-list">
          {#each calls as c (c.id)}
            <li>
              <span class="c-ico" class:missed={c.kind === 'missed'}>
                <Icon
                  name={c.kind === 'incoming'
                    ? 'phone-in'
                    : c.kind === 'outgoing'
                      ? 'phone-out'
                      : 'phone-missed'}
                  size={15}
                />
              </span>
              <div class="c-info">
                <strong translate="no">{c.name}</strong>
                <span class="sub tabular">
                  {c.number} · {formatCallDuration(c.duration_secs)} · {relativeMinutes(c.date_ms)}
                </span>
              </div>
              <button class="plain sm" onclick={() => dial(c.number)}>Call</button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="panel-box">
      <div class="bar">
        <h3>Contacts ({contacts.length})</h3>
        <div class="bar-acts">
          <input type="search" placeholder="Search contacts…" bind:value={contactQuery} aria-label="Search contacts" />
          <button class="primary" disabled={!serial || contacts.length === 0} onclick={exportContacts}>
            <Icon name="download" size={14} /> Export .vcf
          </button>
        </div>
      </div>
      {#if !serial}
        <p class="sub">Connect your Android phone to browse and export contacts.</p>
      {:else if contactsNote}
        <p class="warn">{contactsNote}</p>
      {:else if filteredContacts.length === 0}
        <p class="sub">No matching contacts.</p>
      {:else}
        <ul class="call-list">
          {#each filteredContacts.slice(0, 80) as ct (ct.name + ct.number)}
            <li>
              <span class="avatar" style:background={appBadgeColor(ct.name)}>
                {ct.name.slice(0, 1).toUpperCase()}
              </span>
              <div class="c-info">
                <strong translate="no">{ct.name}</strong>
                <span class="sub tabular" translate="no">{ct.number}</span>
              </div>
              <button class="plain sm" onclick={() => dial(ct.number)}>Call</button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>

{:else if tab === 'photos'}
  <section class="panel-box">
    <div class="bar">
      <div>
        <h3>Photos</h3>
        <p class="sub">Browse recent photos from your phone and save them straight to your computer.</p>
      </div>
      <div class="bar-acts">
        <button
          class="primary"
          disabled={!serial || filteredPhotos.length === 0 || pullingAllPhotos}
          onclick={pullAllShownPhotos}
        >
          <Icon name="download" size={14} />
          {pullingAllPhotos ? 'Downloading…' : `Backup all (${filteredPhotos.length})`}
        </button>
        <button class="plain" disabled={!serial} onclick={onScreenshot}>
          <Icon name="camera" size={14} /> Phone screenshot
        </button>
        <button class="plain" disabled={!serial || photosLoading} onclick={loadPhotos}>
          <Icon name="refresh" size={14} /> Refresh
        </button>
      </div>
    </div>

    {#if !serial}
      <p class="sub">Connect your Android phone to browse and download photos.</p>
    {:else}
      <div class="filter-pills">
        {#each photoFolders as f (f)}
          <button class:on={photoFolder === f} onclick={() => (photoFolder = f)}>{f}</button>
        {/each}
      </div>

      {#if filteredPhotos.length === 0}
        <p class="sub">{photosLoading ? 'Scanning photos on your phone…' : 'No photos found in this folder.'}</p>
      {:else}
        <div class="photo-grid">
          {#each filteredPhotos as p (p.path)}
            <div class="photo-tile">
              <button
                class="thumb-btn"
                onclick={() => {
                  void fetchPreview(p.path);
                  lightbox = p;
                }}
              >
                {#if previews[p.path]}
                  <img src={previews[p.path]} alt={p.name} />
                {:else}
                  <div class="thumb-placeholder">
                    <Icon name="image" size={20} />
                    <span>{loadingPreview[p.path] ? 'Loading…' : 'Load preview'}</span>
                  </div>
                {/if}
              </button>
              <div class="p-foot">
                <span class="p-name" translate="no" title={p.name}>{p.name}</span>
                <span class="p-sz tabular">{formatBytes(p.size)}</span>
                <button class="icon-sm" title="Save to computer" aria-label="Save to computer" onclick={() => pullPhoto(p)}>
                  <Icon name="download" size={14} />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}

    {#if lightbox}
      <div class="lightbox" role="dialog" aria-label={lightbox.name}>
        <div class="lb-card">
          <div class="bar">
            <strong translate="no">{lightbox.name}</strong>
            <div class="bar-acts">
              <button class="primary" onclick={() => lightbox && pullPhoto(lightbox)}>
                <Icon name="download" size={14} /> Save to PC
              </button>
              <button class="plain danger-btn" onclick={() => lightbox && removePhoto(lightbox)}>
                <Icon name="trash" size={14} /> Delete
              </button>
              <button class="plain" onclick={() => (lightbox = null)}>Close</button>
            </div>
          </div>
          {#if previews[lightbox.path]}
            <img class="lb-img" src={previews[lightbox.path]} alt={lightbox.name} />
          {:else}
            <p class="sub">Loading full preview…</p>
          {/if}
        </div>
      </div>
    {/if}
  </section>

{:else if tab === 'apps'}
  <section class="panel-box">
    <div class="bar">
      <div>
        <h3>Apps &amp; Debloater</h3>
        <p class="sub">Launch apps in phone or tablet windows, extract APKs, force-stop, clear data or disable bloatware.</p>
      </div>
      <div class="bar-acts">
        <input
          type="search"
          placeholder="Search apps…"
          bind:value={appQuery}
          aria-label="Search apps"
        />
        <select bind:value={appDisplaySize} aria-label="Mirrored window shape" title="Window aspect ratio">
          <option value="880x1920">Phone window</option>
          <option value="1600x1000">Tablet window</option>
          <option value="1080x1080">Square window</option>
        </select>
        <button class="primary" disabled={!serial} onclick={installApk}>
          <Icon name="plus" size={14} /> Install APK
        </button>
        <button class="plain" disabled={!serial || appsLoading} onclick={loadApps}>
          <Icon name="refresh" size={14} />
        </button>
      </div>
    </div>

    <div class="filter-pills">
      {#each [['user', 'User apps'], ['system', 'System apps'], ['disabled', 'Disabled'], ['all', 'All']] as [id, lbl] (id)}
        <button
          class:on={appFilter === id}
          disabled={!serial}
          onclick={() => {
            appFilter = id as 'user' | 'system' | 'disabled' | 'all';
            void loadApps();
          }}
        >
          {lbl}
        </button>
      {/each}
    </div>

    {#if !serial}
      <p class="sub">Connect your Android phone to browse and manage installed apps.</p>
    {:else if filteredApps.length === 0}
      <p class="sub">{appsLoading ? 'Loading installed apps…' : 'No matching apps found.'}</p>
    {:else}
      <div class="apps-grid">
        {#each filteredApps as a (a.package)}
          <div class="app-tile" class:disabled-app={a.disabled}>
            <span class="avatar" style:background={appBadgeColor(a.name)}>
              {a.name.slice(0, 1).toUpperCase()}
            </span>
            <div class="a-info">
              <strong translate="no">
                {a.name}
                {#if a.disabled}<em class="tag-sm">disabled</em>{/if}
              </strong>
              <span class="a-pkg" translate="no" title={a.package}>{a.package}</span>
            </div>
            <div class="a-acts">
              <button
                class="plain sm"
                title="Open in mirrored window"
                onclick={() => onMirrorApp(a.package, appDisplaySize)}
              >
                <Icon name="cast" size={13} /> Window
              </button>
              <button
                class="plain sm"
                title="Launch on phone"
                onclick={() => serial && void mobileIpc.launchApp(serial, a.package, false).then(() => onNotice(`Opened ${a.name}`))}
              >
                Open
              </button>
              <button
                class="icon-sm"
                title="Backup APK to computer"
                aria-label={`Backup ${a.name} APK`}
                onclick={() => void runAppAction(a, 'pull_apk')}
              >
                <Icon name="download" size={13} />
              </button>
              <button
                class="icon-sm"
                title="Force stop"
                aria-label={`Force stop ${a.name}`}
                onclick={() => void runAppAction(a, 'force_stop')}
              >
                <Icon name="power" size={13} />
              </button>
              <button
                class="icon-sm"
                title={a.disabled ? 'Re-enable app' : 'Disable app (no root)'}
                aria-label={a.disabled ? `Enable ${a.name}` : `Disable ${a.name}`}
                onclick={() => void runAppAction(a, a.disabled ? 'enable' : 'disable')}
              >
                <Icon name={a.disabled ? 'eye' : 'eye-off'} size={13} />
              </button>
              {#if !a.system}
                <button
                  class="icon-sm"
                  title="Uninstall"
                  aria-label={`Uninstall ${a.name}`}
                  onclick={() => uninstallApp(a)}
                >
                  <Icon name="trash" size={13} />
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>

{:else if tab === 'deck'}
  <section class="panel-box">
    <div class="bar">
      <div>
        <h3>Deck</h3>
        <p class="sub">One-click remote buttons for your phone. Add shortcuts for apps, links, keys, or ADB commands.</p>
      </div>
      <button class="primary" onclick={() => (addingDeck = !addingDeck)}>
        <Icon name="plus" size={14} /> {addingDeck ? 'Cancel' : 'New button'}
      </button>
    </div>

    {#if addingDeck}
      <form
        class="deck-form"
        onsubmit={(e) => {
          e.preventDefault();
          void saveDeck();
        }}
      >
        <input type="text" placeholder="Button name (e.g. Open Spotify)" bind:value={newDeckTitle} />
        <select bind:value={newDeckKind}>
          <option value="key">Android key</option>
          <option value="app">Open app (package)</option>
          <option value="url">Open link on phone</option>
          <option value="screenshot">Phone screenshot</option>
          <option value="shell">ADB shell command</option>
        </select>
        {#if newDeckKind !== 'screenshot'}
          <input
            type="text"
            placeholder={newDeckKind === 'key'
              ? 'play_pause, next, wakeup, sleep, home, back…'
              : newDeckKind === 'app'
                ? 'com.spotify.music'
                : newDeckKind === 'url'
                  ? 'https://example.com'
                  : 'input keyevent 26'}
            bind:value={newDeckPayload}
          />
        {/if}
        <button type="submit" class="primary" disabled={!newDeckTitle.trim()}>Save</button>
      </form>
    {/if}

    <div class="deck-grid">
      {#each deck as item (item.id)}
        <div class="deck-tile">
          <button class="deck-run" disabled={!serial} onclick={() => runDeck(item)}>
            <span class="deck-ico"><Icon name={item.icon} size={18} /></span>
            <strong translate="no">{item.title}</strong>
            <span class="sub">{item.kind}</span>
          </button>
          {#if !item.id.startsWith('preset-')}
            <button
              class="deck-del"
              aria-label="Delete button"
              onclick={async () => (deck = await mobileIpc.deckDelete(item.id))}
            >
              <Icon name="x" size={12} />
            </button>
          {/if}
        </div>
      {/each}
    </div>
  </section>

{:else if tab === 'guide'}
  <div class="two-col">
    <section class="panel-box">
      <h3>1. Connect with USB cable</h3>
      <ol class="steps">
        <li>On your Android phone, open <strong>Settings → About phone</strong> and tap <strong>Build number</strong> 7 times to enable Developer options.</li>
        <li>Open <strong>Settings → Developer options</strong> and turn on <strong>USB debugging</strong>.</li>
        <li>Plug your phone into this computer and tap <strong>Allow</strong> on the authorization prompt.</li>
      </ol>
      <h3>2. Go wireless (Wi-Fi)</h3>
      <ol class="steps">
        <li>On Android 11+, open <strong>Developer options → Wireless debugging</strong> and turn it on.</li>
        <li>Tap <strong>Pair device with pairing code</strong>, then click the <strong>+</strong> button in NomNom Mobile Manager to enter the IP:port and 6-digit code.</li>
        <li>Or connect once via USB, click <strong>+</strong>, and choose <strong>Switch to Wi-Fi</strong> — then unplug the cable.</li>
      </ol>
      <button class="primary" onclick={onOpenPair}><Icon name="wifi" size={14} /> Open Wi-Fi pairing</button>
    </section>

    <section class="panel-box">
      <h3>Android tools status</h3>
      <p class="sub">NomNom uses the official ADB and scrcpy binaries locally. Nothing goes through the cloud.</p>
      <div class="status-list">
        <div class="st-row">
          <span>ADB</span>
          <strong class:ok={tools?.adb_found}>{tools?.adb_found ? 'Installed' : 'Not installed'}</strong>
        </div>
        <div class="st-row">
          <span>scrcpy</span>
          <strong class:ok={tools?.scrcpy_found}>{tools?.scrcpy_found ? 'Installed' : 'Not installed'}</strong>
        </div>
      </div>
      {#if !tools?.adb_found || !tools?.scrcpy_found}
        <button class="primary" disabled={tools?.installing} onclick={onInstallTools}>
          <Icon name="cloud-download" size={15} />
          {tools?.installing ? 'Downloading Android tools…' : 'Install ADB & scrcpy (~30 MB)'}
        </button>
      {:else}
        <p class="sub" translate="no">Path: {tools.scrcpy_path}</p>
      {/if}
    </section>
  </div>

{:else if tab === 'settings'}
  <div class="two-col">
    <section class="panel-box">
      <h3>Screen mirroring quality</h3>
      <div class="set-row">
        <div>
          <strong>Max resolution</strong>
          <p class="sub">Lower values reduce Wi-Fi latency.</p>
        </div>
        <select
          value={mirrorOpts.maxSize ?? 1280}
          onchange={(e) => (mirrorOpts = { ...mirrorOpts, maxSize: Number(e.currentTarget.value) })}
        >
          <option value={0}>Native resolution</option>
          <option value={1920}>1920p (Full HD)</option>
          <option value={1280}>1280p (Balanced)</option>
          <option value={1024}>1024p (Fast)</option>
        </select>
      </div>
      <div class="set-row">
        <div>
          <strong>Video bitrate</strong>
          <p class="sub">Higher bitrate looks sharper on Wi-Fi 5/6.</p>
        </div>
        <select
          value={mirrorOpts.bitRateMbps ?? 8}
          onchange={(e) => (mirrorOpts = { ...mirrorOpts, bitRateMbps: Number(e.currentTarget.value) })}
        >
          <option value={4}>4 Mbps</option>
          <option value={8}>8 Mbps (Recommended)</option>
          <option value={16}>16 Mbps</option>
          <option value={24}>24 Mbps</option>
        </select>
      </div>
      <div class="set-row">
        <div>
          <strong>Max frame rate</strong>
          <p class="sub">60 FPS feels smooth while keeping CPU low.</p>
        </div>
        <select
          value={mirrorOpts.maxFps ?? 60}
          onchange={(e) => (mirrorOpts = { ...mirrorOpts, maxFps: Number(e.currentTarget.value) })}
        >
          <option value={30}>30 FPS</option>
          <option value={60}>60 FPS</option>
          <option value={120}>120 FPS</option>
        </select>
      </div>
      <div class="set-row">
        <div>
          <strong>Forward phone audio while mirroring</strong>
          <p class="sub">Requires Android 11 or newer.</p>
        </div>
        <Switch
          checked={mirrorOpts.audio ?? true}
          label="Forward phone audio while mirroring"
          onchange={(v) => (mirrorOpts = { ...mirrorOpts, audio: v })}
        />
      </div>
      <div class="set-row">
        <div>
          <strong>Turn phone screen off while mirroring</strong>
          <p class="sub">Saves battery and keeps your phone private on the desk.</p>
        </div>
        <Switch
          checked={mirrorOpts.turnScreenOff ?? false}
          label="Turn phone screen off while mirroring"
          onchange={(v) => (mirrorOpts = { ...mirrorOpts, turnScreenOff: v })}
        />
      </div>
      <div class="set-row">
        <div>
          <strong>Keep phone awake while connected</strong>
          <p class="sub">Prevents the screen from locking during a session.</p>
        </div>
        <Switch
          checked={mirrorOpts.stayAwake ?? true}
          label="Keep phone awake while connected"
          onchange={(v) => (mirrorOpts = { ...mirrorOpts, stayAwake: v })}
        />
      </div>
    </section>

    <section class="panel-box">
      <h3>Muted notification apps</h3>
      <p class="sub">Apps you muted from the Home notifications card.</p>
      {#if !snapshot || snapshot.muted_packages.length === 0}
        <p class="sub">No muted apps.</p>
      {:else}
        <ul class="muted-list">
          {#each snapshot.muted_packages as pkg (pkg)}
            <li>
              <span translate="no">{pkg}</span>
              <button class="plain sm" onclick={() => onUnmuteApp(pkg)}>Unmute</button>
            </li>
          {/each}
        </ul>
      {/if}

      <hr />
      <h3>No-cable QR file transfer</h3>
      <p class="sub">Need to send files to an iPhone or a phone without USB debugging? Use NomNom's built-in Wi-Fi QR transfer.</p>
      <button class="plain" onclick={() => app.go('phone')}>
        <Icon name="smartphone" size={14} /> Open Phone transfer (QR)
      </button>
    </section>
  </div>
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
  .two-col {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 14px;
    align-items: start;
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .bar-acts {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  h3 { margin: 0; font-size: 15px; }
  .sub { margin: 2px 0 0; font-size: 12.5px; color: var(--text-3); }
  .pad { padding: 10px; }
  .warn { margin: 0; padding: 9px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border-strong); color: var(--accent-text); font-size: 12.5px; }
  hr { border: 0; border-top: 1px solid var(--border); margin: 4px 0; }
  input[type='text'], input[type='search'], select {
    padding: 7px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: var(--text); font-size: 13px;
  }
  .plain, .primary {
    display: inline-flex; align-items: center; gap: 6px; padding: 7px 13px;
    border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: inherit; font-weight: 600; font-size: 12.5px; cursor: pointer;
  }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .primary:hover:not(:disabled) { background: var(--accent-hover); }
  .plain.sm { padding: 4px 9px; font-size: 12px; }
  .ok-btn { color: var(--ok); }
  .danger-btn { color: var(--danger); }
  button:disabled { opacity: 0.45; cursor: default; }
  .icon-sm { border: 0; background: none; color: var(--text-3); padding: 4px; border-radius: 6px; display: grid; cursor: pointer; }
  .icon-sm:hover { background: var(--surface-3); color: var(--text); }

  /* Messages */
  .msg-split { display: grid; grid-template-columns: 260px minmax(0, 1fr); gap: 12px; min-height: 360px; }
  @media (max-width: 780px) { .msg-split { grid-template-columns: 1fr; } }
  .thread-list { display: grid; align-content: start; gap: 4px; border-right: 1px solid var(--border); padding-right: 10px; max-height: 440px; overflow: auto; }
  .new-sms input { width: 100%; box-sizing: border-box; margin-bottom: 6px; }
  .thread-row { display: flex; align-items: center; gap: 9px; padding: 8px 9px; border: 0; border-radius: var(--radius-s); background: transparent; text-align: left; cursor: pointer; }
  .thread-row:hover, .thread-row.on { background: var(--surface-2); }
  .avatar { width: 32px; height: 32px; flex: none; border-radius: 50%; color: #fff; font-weight: 700; font-size: 12px; display: grid; place-items: center; }
  .t-meta { flex: 1; min-width: 0; display: grid; }
  .t-meta strong { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .t-snip { font-size: 11.5px; color: var(--text-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .t-when { font-size: 11px; color: var(--text-3); flex: none; }
  .chat-pane { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  .chat-log { flex: 1; display: flex; flex-direction: column; gap: 8px; max-height: 380px; overflow: auto; padding: 6px 2px; }
  .bubble { align-self: flex-start; max-width: 78%; padding: 8px 12px; border-radius: var(--radius-m); background: var(--surface-2); border: 1px solid var(--border); display: grid; gap: 3px; font-size: 13px; }
  .bubble.out { align-self: flex-end; background: var(--accent-soft); border-color: var(--accent); }
  .bubble small { font-size: 10.5px; color: var(--text-3); }
  .chat-form { display: flex; gap: 8px; }
  .chat-form input { flex: 1; }

  /* Calls */
  .dial-display { display: flex; gap: 8px; }
  .dial-display input { flex: 1; font-size: 15px; font-weight: 600; }
  .keypad { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .kcell { padding: 12px; border-radius: var(--radius-s); border: 1px solid var(--border); background: var(--surface-2); font-size: 16px; font-weight: 700; cursor: pointer; }
  .kcell:hover { background: var(--surface-3); }
  .call-ctrls { display: flex; gap: 8px; flex-wrap: wrap; }
  .call-list { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; max-height: 400px; overflow: auto; }
  .call-list li { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); }
  .c-ico { width: 30px; height: 30px; border-radius: 50%; background: var(--surface-1); display: grid; place-items: center; color: var(--ok); }
  .c-ico.missed { color: var(--danger); }
  .c-info { flex: 1; min-width: 0; display: grid; }
  .c-info strong { font-size: 13px; }

  /* Photos */
  .filter-pills { display: flex; gap: 6px; flex-wrap: wrap; }
  .filter-pills button { padding: 4px 11px; border-radius: var(--radius-s); border: 1px solid var(--border); background: var(--surface-2); color: var(--text-2); font-size: 12px; font-weight: 600; cursor: pointer; }
  .filter-pills button.on { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .photo-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(156px, 1fr)); gap: 10px; }
  .photo-tile { border-radius: var(--radius-m); overflow: hidden; background: var(--surface-2); border: 1px solid var(--border); display: grid; }
  .thumb-btn { border: 0; padding: 0; background: var(--surface-3); aspect-ratio: 1; cursor: pointer; overflow: hidden; display: grid; place-items: center; }
  .thumb-btn img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .thumb-placeholder { display: grid; place-items: center; gap: 4px; color: var(--text-3); font-size: 11px; }
  .p-foot { display: flex; align-items: center; gap: 6px; padding: 6px 8px; font-size: 11.5px; }
  .p-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .p-sz { color: var(--text-3); flex: none; }
  .lightbox { position: fixed; inset: 0; z-index: 60; background: rgba(0, 0, 0, 0.72); display: grid; place-items: center; padding: 24px; }
  .lb-card { width: min(760px, 92vw); background: var(--surface-1); border: 1px solid var(--border-strong); border-radius: var(--radius-m); padding: 14px 16px; display: grid; gap: 12px; }
  .lb-img { max-width: 100%; max-height: 70vh; object-fit: contain; margin: 0 auto; border-radius: var(--radius-s); }

  /* Apps */
  .apps-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(330px, 1fr)); gap: 8px; max-height: 520px; overflow: auto; }
  .app-tile { display: flex; align-items: center; gap: 10px; padding: 9px 11px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .app-tile.disabled-app { opacity: 0.65; border-style: dashed; }
  .tag-sm { font-style: normal; font-size: 10.5px; color: var(--text-3); margin-left: 5px; font-weight: 500; }
  .a-info { flex: 1; min-width: 0; display: grid; }
  .a-info strong { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .a-pkg { font-size: 11px; color: var(--text-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .a-acts { display: flex; align-items: center; gap: 4px; flex: none; }

  /* Deck */
  .deck-form { display: flex; gap: 8px; flex-wrap: wrap; padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .deck-form input { flex: 1; min-width: 160px; }
  .deck-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 10px; }
  .deck-tile { position: relative; }
  .deck-run { width: 100%; display: grid; gap: 6px; padding: 14px; border-radius: var(--radius-m); border: 1px solid var(--border); background: var(--surface-2); text-align: left; cursor: pointer; }
  .deck-run:hover:not(:disabled) { background: var(--surface-3); border-color: var(--border-strong); }
  .deck-ico { width: 34px; height: 34px; border-radius: 10px; background: var(--accent-soft); color: var(--accent-text); display: grid; place-items: center; }
  .deck-del { position: absolute; top: 6px; right: 6px; width: 22px; height: 22px; border: 0; border-radius: 50%; background: var(--surface-1); color: var(--text-3); display: grid; place-items: center; cursor: pointer; }
  .deck-del:hover { color: var(--danger); }

  /* Guide & Settings */
  .steps { margin: 4px 0 8px; padding-left: 20px; display: grid; gap: 6px; font-size: 13px; color: var(--text-2); line-height: 1.45; }
  .status-list { display: grid; gap: 6px; }
  .st-row { display: flex; justify-content: space-between; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); font-size: 13px; }
  .st-row strong.ok { color: var(--ok); }
  .set-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 8px 0; border-top: 1px solid var(--border); }
  .set-row:first-of-type { border-top: 0; }
  .muted-list { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  .muted-list li { display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: var(--radius-s); background: var(--surface-2); font-size: 12.5px; }
</style>
