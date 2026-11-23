<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import type {
    MobileDevice,
    MobileNotification,
    MobileSnapshot,
    MobileTimelineEntry,
  } from '../../lib/ipc';
  import PhoneMockup from './PhoneMockup.svelte';
  import {
    appBadgeColor,
    formatTrackTime,
    relativeMinutes,
    timelineIcon,
  } from './logic';

  let {
    device,
    snapshot,
    timeline,
    busyAction,
    onMirror,
    onOpenAppPicker,
    onSendFiles,
    onOpenSavedFolder,
    onSendClipboard,
    onFindPhone,
    onMediaAction,
    onToggleControl,
    onAudioToggle,
    onMicToggle,
    onRecordToggle,
    onSwipe,
    onOpenSoundDisplay,
    onContinuityCamera,
    onWebcam,
    onTypeText,
    onKeyEvent,
    onNotificationAction,
    onOpenHistory,
    onOpenPair,
  }: {
    device: MobileDevice | null;
    snapshot: MobileSnapshot | null;
    timeline: MobileTimelineEntry[];
    busyAction: string;
    onMirror: () => void;
    onOpenAppPicker: () => void;
    onSendFiles: () => void;
    onOpenSavedFolder: () => void;
    onSendClipboard: () => void;
    onFindPhone: () => void;
    onMediaAction: (act: 'play_pause' | 'next' | 'prev') => void;
    onToggleControl: (ctrl: 'dnd' | 'torch' | 'wifi' | 'bluetooth' | 'hotspot', next: boolean) => void;
    onAudioToggle: (next: boolean) => void;
    onMicToggle: (next: boolean) => void;
    onRecordToggle: () => void;
    onSwipe: (dir: 'up' | 'down' | 'left' | 'right' | 'notifications' | 'quicksettings') => void;
    onOpenSoundDisplay: () => void;
    onContinuityCamera: () => void;
    onWebcam: (facing: 'back' | 'front') => void;
    onTypeText: (text: string) => Promise<void>;
    onKeyEvent: (key: string) => void;
    onNotificationAction: (
      n: MobileNotification | null,
      act: 'dismiss' | 'clear_all' | 'mute' | 'open' | 'reply',
      replyText?: string,
    ) => void;
    onOpenHistory: () => void;
    onOpenPair: () => void;
  } = $props();

  let typingOpen = $state(false);
  let liveInput = $state('');
  let replyKey = $state<string | null>(null);
  let replyDraft = $state('');
  let menuKey = $state<string | null>(null);
  let camFacing = $state<'back' | 'front'>('back');

  const connected = $derived(!!device && device.state === 'device');
  const media = $derived(snapshot?.media ?? null);
  const mediaPct = $derived(
    media && media.duration_ms > 0
      ? Math.max(0, Math.min(100, Math.round((media.position_ms / media.duration_ms) * 100)))
      : 0,
  );
  const controls = $derived(snapshot?.controls ?? {
    dnd: false,
    torch: false,
    wifi: false,
    bluetooth: false,
    hotspot: false,
    audio_forwarding: false,
    mic_active: false,
    webcam_active: false,
    recording_active: false,
    dark_mode: false,
  });
  const notifs = $derived(snapshot?.notifications ?? []);
  const recentTimeline = $derived(timeline.slice(0, 5));

  async function sendTyped() {
    const val = liveInput;
    if (!val.trim()) return;
    liveInput = '';
    await onTypeText(val);
  }

  function submitReply(n: MobileNotification) {
    onNotificationAction(n, 'reply', replyDraft);
    replyKey = null;
    replyDraft = '';
  }
</script>

<div class="home-grid">
  <!-- LEFT COLUMN -->
  <div class="col-main">
    <!-- Hero Device + Media Card -->
    <section class="card hero-card">
      <div class="device-side">
        <PhoneMockup
          level={snapshot?.battery.level ?? 0}
          charging={snapshot?.battery.charging ?? false}
          {connected}
          transport={snapshot?.transport ?? device?.transport ?? 'wifi'}
        />

        <div class="device-meta">
          {#if connected && snapshot}
            <h2 class="dev-name" translate="no">{snapshot.name}</h2>
            <p class="dev-os">
              {snapshot.android_version ? `Android ${snapshot.android_version}` : 'Android device'}
            </p>
            <div class="badges">
              <span class="chip">
                <Icon name={snapshot.transport === 'wifi' ? 'wifi' : 'usb'} size={13} />
                {#if snapshot.transport === 'wifi'}
                  Wi-Fi{snapshot.latency_ms ? ` · ${snapshot.latency_ms} ms` : ''}
                {:else}
                  USB · Connected
                {/if}
              </span>
              <span class="chip">
                <Icon name={snapshot.battery.charging ? 'zap' : 'battery-charging'} size={13} />
                {snapshot.battery.charging
                  ? snapshot.battery.charge_type
                  : `${snapshot.battery.level}% battery`}
              </span>
            </div>
          {:else if device && device.state === 'unauthorized'}
            <h2 class="dev-name" translate="no">{device.model}</h2>
            <p class="dev-os">Tap Allow on your phone to authorize USB debugging.</p>
            <div class="badges">
              <span class="chip warn"><Icon name="lock" size={13} /> Waiting for permission</span>
            </div>
          {:else}
            <h2 class="dev-name">No phone connected</h2>
            <p class="dev-os">Connect with USB or pair over Wi-Fi to mirror, control and sync.</p>
            <div class="badges">
              <button class="chip action" onclick={onOpenPair}>
                <Icon name="plus" size={13} /> Pair device
              </button>
            </div>
          {/if}
        </div>
      </div>

      <!-- Media Widget -->
      <div class="media-card">
        <div class="media-top">
          <span class="media-source">
            <Icon name="music" size={13} />
            <span>Phone</span>
            {#if media}
              <strong class="media-app" translate="no">{media.app}</strong>
            {/if}
          </span>
          <span class="media-badge">
            <Icon name={media?.playing ? 'volume' : 'pause'} size={13} />
          </span>
        </div>

        <div class="media-body">
          <div
            class="art"
            style:background={media ? appBadgeColor(media.app) : 'var(--surface-3)'}
          >
            <Icon name="music" size={18} />
          </div>
          <div class="track-info">
            <div class="track-title" translate="no">
              {media ? media.title : connected ? 'Nothing playing' : 'Connect a phone'}
            </div>
            <div class="track-artist" translate="no">
              {media
                ? media.artist || media.app
                : connected
                  ? 'Media controls appear when audio plays'
                  : 'Playback controls ready'}
            </div>
          </div>
          <div class="media-btns">
            <button
              class="mbtn"
              disabled={!connected}
              aria-label="Previous track"
              onclick={() => onMediaAction('prev')}
            >
              <Icon name="skip-back" size={15} />
            </button>
            <button
              class="mbtn play"
              disabled={!connected}
              aria-label={media?.playing ? 'Pause' : 'Play'}
              onclick={() => onMediaAction('play_pause')}
            >
              <Icon name={media?.playing ? 'pause' : 'play'} size={15} />
            </button>
            <button
              class="mbtn"
              disabled={!connected}
              aria-label="Next track"
              onclick={() => onMediaAction('next')}
            >
              <Icon name="skip-forward" size={15} />
            </button>
          </div>
        </div>

        <div class="media-progress">
          <div class="bar"><i style:width="{mediaPct}%"></i></div>
          <div class="times tabular">
            <span>{media ? formatTrackTime(media.position_ms) : '0:00'}</span>
            <span>
              {media && media.duration_ms > 0
                ? `-${formatTrackTime(Math.max(0, media.duration_ms - media.position_ms))}`
                : '0:00'}
            </span>
          </div>
        </div>
      </div>
    </section>

    <!-- 4 Quick Action Cards -->
    <section class="quick-grid" aria-label="Quick actions">
      <div class="qcard">
        <div class="qtop">
          <button
            class="qmain-ico"
            disabled={!connected || busyAction === 'mirror'}
            onclick={onMirror}
            aria-label="Mirror screen"
          >
            <Icon name="cast" size={18} />
          </button>
          <button
            class="qmini"
            disabled={!connected}
            title="Open single app window"
            aria-label="Open single app window"
            onclick={onOpenAppPicker}
          >
            <Icon name="grid" size={15} />
          </button>
        </div>
        <button class="qlabel" disabled={!connected || busyAction === 'mirror'} onclick={onMirror}>
          Mirror screen
        </button>
      </div>

      <div class="qcard">
        <div class="qtop">
          <button
            class="qmain-ico"
            disabled={!connected || busyAction === 'files'}
            onclick={onSendFiles}
            aria-label="Send files"
          >
            <Icon name="send" size={18} />
          </button>
          <button
            class="qmini"
            title="Open received files folder"
            aria-label="Open received files folder"
            onclick={onOpenSavedFolder}
          >
            <Icon name="folder" size={15} />
          </button>
        </div>
        <button class="qlabel" disabled={!connected || busyAction === 'files'} onclick={onSendFiles}>
          {busyAction === 'files' ? 'Sending…' : 'Send files'}
        </button>
      </div>

      <div class="qcard">
        <div class="qtop">
          <button
            class="qmain-ico"
            disabled={!connected || busyAction === 'clipboard'}
            onclick={onSendClipboard}
            aria-label="Send clipboard"
          >
            <Icon name="clipboard" size={18} />
          </button>
          <button
            class="qmini"
            disabled={!connected}
            title="Send computer clipboard now"
            aria-label="Send computer clipboard now"
            onclick={onSendClipboard}
          >
            <Icon name="copy" size={15} />
          </button>
        </div>
        <button
          class="qlabel"
          disabled={!connected || busyAction === 'clipboard'}
          onclick={onSendClipboard}
        >
          Send clipboard
        </button>
      </div>

      <div class="qcard">
        <div class="qtop">
          <button
            class="qmain-ico"
            disabled={!connected || busyAction === 'find'}
            onclick={onFindPhone}
            aria-label="Find phone"
          >
            <Icon name="bell-ring" size={18} />
          </button>
        </div>
        <button class="qlabel" disabled={!connected || busyAction === 'find'} onclick={onFindPhone}>
          {busyAction === 'find' ? 'Ringing…' : 'Find phone'}
        </button>
      </div>
    </section>

    <!-- Notifications Card -->
    <section class="card notif-card" aria-labelledby="mob-notifs">
      <div class="card-head">
        <h3 id="mob-notifs">Notifications · {notifs.length}</h3>
        <div class="head-actions">
          <button class="ghost-btn" onclick={onOpenHistory}>
            <Icon name="clock" size={14} /> History
          </button>
          <button
            class="ghost-btn"
            disabled={!connected || notifs.length === 0}
            onclick={() => onNotificationAction(null, 'clear_all')}
          >
            <Icon name="x" size={14} /> Clear all
          </button>
        </div>
      </div>

      {#if !connected}
        <p class="empty">Connect your Android phone to read and reply to notifications here.</p>
      {:else if notifs.length === 0}
        <p class="empty">All caught up. Active notifications from your phone show up here.</p>
      {:else}
        <ul class="notif-list">
          {#each notifs as n (n.key)}
            <li class="notif-item">
              <div class="n-avatar" style:background={appBadgeColor(n.app_name)}>
                {n.app_name.slice(0, 1).toUpperCase()}
              </div>
              <div class="n-body">
                <div class="n-meta">
                  <span translate="no">{n.app_name}</span>
                  <span>·</span>
                  <span>{relativeMinutes(n.post_time_ms)}</span>
                </div>
                <div class="n-title" translate="no">{n.title}</div>
                {#if n.text}
                  <div class="n-text" translate="no">{n.text}</div>
                {/if}

                <div class="n-actions">
                  <button
                    class="pill-act"
                    onclick={() => {
                      replyKey = replyKey === n.key ? null : n.key;
                      replyDraft = '';
                    }}
                  >
                    Reply
                  </button>
                  <button class="pill-act" onclick={() => onNotificationAction(n, 'dismiss')}>
                    Mark as read
                  </button>
                  <button class="pill-act" onclick={() => onNotificationAction(n, 'mute')}>
                    Mute
                  </button>
                </div>

                {#if replyKey === n.key}
                  <form
                    class="reply-bar"
                    onsubmit={(e) => {
                      e.preventDefault();
                      submitReply(n);
                    }}
                  >
                    <input
                      type="text"
                      bind:value={replyDraft}
                      placeholder={`Reply to ${n.title}…`}
                      aria-label="Reply text"
                    />
                    <button type="submit" class="primary-sm" disabled={!replyDraft.trim()}>
                      Send
                    </button>
                  </form>
                {/if}
              </div>
              <div class="n-tools">
                <button
                  class="icon-btn"
                  title="Open app on phone"
                  aria-label="Open app on phone"
                  onclick={() => {
                    menuKey = menuKey === n.key ? null : n.key;
                    onNotificationAction(n, 'open');
                  }}
                >
                  <Icon name="ellipsis" size={15} />
                </button>
                <button
                  class="icon-btn"
                  title="Dismiss"
                  aria-label="Dismiss notification"
                  onclick={() => onNotificationAction(n, 'dismiss')}
                >
                  <Icon name="x" size={15} />
                </button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>

  <!-- RIGHT COLUMN -->
  <div class="col-side">
    <!-- Phone Controls Card -->
    <section class="card controls-card" aria-labelledby="mob-ctrls">
      <div class="card-head">
        <h3 id="mob-ctrls">Phone controls</h3>
        <button class="link-btn" disabled={!connected} onclick={onOpenSoundDisplay}>
          Sound & display
        </button>
      </div>

      <div class="toggles-row">
        <button
          class="t-circle"
          class:on={controls.dnd}
          disabled={!connected}
          onclick={() => onToggleControl('dnd', !controls.dnd)}
        >
          <Icon name="moon" size={17} />
          <span>DND</span>
        </button>
        <button
          class="t-circle"
          class:on={controls.torch}
          disabled={!connected}
          onclick={() => onToggleControl('torch', !controls.torch)}
        >
          <Icon name="flashlight" size={17} />
          <span>Torch</span>
        </button>
        <button
          class="t-circle"
          class:on={controls.wifi}
          disabled={!connected}
          onclick={() => onToggleControl('wifi', !controls.wifi)}
        >
          <Icon name="wifi" size={17} />
          <span>Wi-Fi</span>
        </button>
        <button
          class="t-circle"
          class:on={controls.bluetooth}
          disabled={!connected}
          onclick={() => onToggleControl('bluetooth', !controls.bluetooth)}
        >
          <Icon name="bluetooth" size={17} />
          <span>BT</span>
        </button>
        <button
          class="t-circle"
          class:on={controls.hotspot}
          disabled={!connected}
          onclick={() => onToggleControl('hotspot', !controls.hotspot)}
        >
          <Icon name="radio" size={17} />
          <span>Hotspot</span>
        </button>
      </div>

      <div class="speaker-row">
        <span class="sp-label">
          <Icon name="volume" size={16} />
          <span>Phone as speaker</span>
        </span>
        <Switch
          checked={controls.audio_forwarding}
          disabled={!connected}
          label="Phone as speaker"
          onchange={(v) => onAudioToggle(v)}
        />
      </div>

      <div class="speaker-row">
        <span class="sp-label">
          <Icon name="mic" size={16} />
          <span>Phone as microphone</span>
        </span>
        <Switch
          checked={controls.mic_active}
          disabled={!connected}
          label="Phone as microphone"
          onchange={(v) => onMicToggle(v)}
        />
      </div>
    </section>

    <!-- Continuity Camera + Webcam Row -->
    <section class="cam-grid" aria-label="Camera tools">
      <button
        class="card cam-card"
        disabled={!connected || busyAction === 'camera'}
        onclick={onContinuityCamera}
      >
        <div class="qtop">
          <span class="qmain-ico static"><Icon name="camera" size={17} /></span>
          <span class="qmini-ico"><Icon name="image" size={15} /></span>
        </div>
        <span class="cam-title">
          {busyAction === 'camera' ? 'Waiting for photo…' : 'Continuity Camera'}
        </span>
      </button>

      <div class="card cam-card" class:active={controls.webcam_active}>
        <div class="qtop">
          <button
            class="qmain-ico"
            disabled={!connected || busyAction === 'webcam'}
            aria-label="Phone as webcam"
            onclick={() => onWebcam(camFacing)}
          >
            <Icon name="video" size={17} />
          </button>
          <button
            class="face-pill"
            disabled={!connected}
            title="Switch front or back camera"
            onclick={() => (camFacing = camFacing === 'back' ? 'front' : 'back')}
          >
            {camFacing === 'back' ? 'Back' : 'Front'}
          </button>
        </div>
        <button
          class="qlabel"
          disabled={!connected || busyAction === 'webcam'}
          onclick={() => onWebcam(camFacing)}
        >
          {controls.webcam_active ? 'Stop webcam' : 'Phone as webcam'}
        </button>
      </div>
    </section>

    <!-- Record Screen + Type on Phone Card -->
    <section class="card type-card">
      <div class="type-head">
        <span class="type-left">
          <span class="qmain-ico static sm"><Icon name="keyboard" size={16} /></span>
          <strong>Type &amp; remote control</strong>
        </span>
        <div class="head-actions">
          <button
            class="start-btn"
            class:rec-btn={controls.recording_active}
            disabled={!connected}
            title="Record phone screen to MKV video"
            onclick={onRecordToggle}
          >
            <Icon name="record" size={13} />
            {controls.recording_active ? 'Stop rec' : 'Rec'}
          </button>
          <button
            class="start-btn"
            disabled={!connected}
            onclick={() => (typingOpen = !typingOpen)}
          >
            {typingOpen ? 'Close' : 'Start'}
          </button>
        </div>
      </div>

      {#if typingOpen && connected}
        <form
          class="type-box"
          onsubmit={(e) => {
            e.preventDefault();
            void sendTyped();
          }}
        >
          <input
            type="text"
            bind:value={liveInput}
            placeholder="Type here and press Enter to send to phone…"
            aria-label="Text to type on phone"
          />
          <button type="submit" class="primary-sm" disabled={!liveInput.trim()}>Send</button>
        </form>
        <div class="key-strip">
          <button class="kbtn" onclick={() => onKeyEvent('enter')}>Enter</button>
          <button class="kbtn" onclick={() => onKeyEvent('backspace')}>Backspace</button>
          <button class="kbtn" onclick={() => onKeyEvent('back')}>Back</button>
          <button class="kbtn" onclick={() => onKeyEvent('home')}>Home</button>
          <button class="kbtn" onclick={() => onKeyEvent('recents')}>Recents</button>
          <button class="kbtn" onclick={() => onKeyEvent('vol_down')}>Vol −</button>
          <button class="kbtn" onclick={() => onKeyEvent('vol_up')}>Vol +</button>
          <button class="kbtn" onclick={() => onSwipe('up')}>Swipe ↑</button>
          <button class="kbtn" onclick={() => onSwipe('down')}>Swipe ↓</button>
          <button class="kbtn" onclick={() => onSwipe('notifications')}>Shade</button>
        </div>
      {/if}
    </section>

    <!-- Timeline Card -->
    <section class="card timeline-card" aria-labelledby="mob-timeline">
      <div class="card-head">
        <h3 id="mob-timeline">Timeline</h3>
        <button class="link-btn" onclick={onOpenHistory}>View all</button>
      </div>

      {#if recentTimeline.length === 0}
        <p class="empty">Mirroring sessions, files, links and controls are logged here.</p>
      {:else}
        <ul class="tl-list">
          {#each recentTimeline as item (item.id)}
            <li>
              <span class="tl-ico"><Icon name={timelineIcon(item.kind)} size={15} /></span>
              <div class="tl-body">
                <div class="tl-title" translate="no">{item.title}</div>
                <div class="tl-sub" translate="no">{item.detail}</div>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
</div>

<style>
  .home-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.55fr) minmax(290px, 0.95fr);
    gap: 14px;
    align-items: start;
  }
  @media (max-width: 960px) {
    .home-grid {
      grid-template-columns: 1fr;
    }
  }
  .col-main,
  .col-side {
    display: grid;
    gap: 12px;
    min-width: 0;
  }

  .card {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    padding: 16px 18px;
    box-shadow: var(--shadow-card);
    min-width: 0;
  }
  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 12px;
  }
  .card-head h3 {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Hero Device + Media */
  .hero-card {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 0.95fr);
    gap: 16px;
    align-items: center;
  }
  @media (max-width: 720px) {
    .hero-card {
      grid-template-columns: 1fr;
    }
  }
  .device-side {
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
  }
  .device-meta {
    min-width: 0;
    display: grid;
    gap: 4px;
  }
  .dev-name {
    font-size: 20px;
    font-weight: 700;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dev-os {
    margin: 0 0 6px;
    font-size: 13px;
    color: var(--text-3);
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }
  .chip.warn {
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .chip.action {
    background: var(--accent);
    color: var(--on-accent);
    border-color: transparent;
    cursor: pointer;
  }
  .chip.action:hover {
    background: var(--accent-hover);
  }

  /* Media Player */
  .media-card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    padding: 12px 14px;
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  .media-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .media-source {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .media-app {
    color: var(--accent-text);
    font-weight: 700;
  }
  .media-badge {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--surface-3);
    display: grid;
    place-items: center;
    color: var(--text-2);
  }
  .media-body {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .art {
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: #fff;
  }
  .track-info {
    flex: 1;
    min-width: 0;
  }
  .track-title {
    font-size: 13px;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track-artist {
    font-size: 11.5px;
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .media-btns {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }
  .mbtn {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text-2);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background var(--dur-1), color var(--dur-1);
  }
  .mbtn:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }
  .mbtn.play {
    width: 32px;
    height: 32px;
    background: var(--surface-3);
    color: var(--text);
    border: 1px solid var(--border-strong);
  }
  .mbtn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .media-progress {
    display: grid;
    gap: 4px;
  }
  .bar {
    height: 3.5px;
    border-radius: 99px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 99px;
    transition: width var(--dur-2) var(--ease-out);
  }
  .times {
    display: flex;
    justify-content: space-between;
    font-size: 10.5px;
    color: var(--text-3);
  }

  /* 4 Quick Action Cards */
  .quick-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }
  @media (max-width: 640px) {
    .quick-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  .qcard {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    padding: 12px 13px;
    display: grid;
    gap: 10px;
    transition: border-color var(--dur-1), background var(--dur-1);
  }
  .qcard:hover {
    border-color: var(--border-strong);
    background: var(--surface-2);
  }
  .qtop {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }
  .qmain-ico {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    border: 0;
    background: var(--accent-soft);
    color: var(--accent-text);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .qmain-ico.static {
    cursor: inherit;
  }
  .qmain-ico.sm {
    width: 30px;
    height: 30px;
    border-radius: 8px;
  }
  .qmain-ico:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .qmini {
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--text-3);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .qmini:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }
  .qmini:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .qmini-ico {
    color: var(--text-3);
    display: grid;
    place-items: center;
  }
  .qlabel {
    border: 0;
    background: none;
    padding: 0;
    text-align: left;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    cursor: pointer;
  }
  .qlabel:disabled {
    opacity: 0.45;
    cursor: default;
  }

  /* Notifications */
  .notif-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .notif-item {
    display: flex;
    align-items: flex-start;
    gap: 11px;
    padding: 11px 12px;
    border-radius: var(--radius-m);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .n-avatar {
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: 50%;
    color: #fff;
    font-weight: 700;
    font-size: 14px;
    display: grid;
    place-items: center;
  }
  .n-body {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 3px;
  }
  .n-meta {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .n-title {
    font-size: 13.5px;
    font-weight: 700;
  }
  .n-text {
    font-size: 12.5px;
    color: var(--text-2);
    line-height: 1.4;
    word-break: break-word;
  }
  .n-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }
  .pill-act {
    border: 1px solid var(--border);
    background: var(--surface-1);
    color: var(--accent-text);
    padding: 3px 10px;
    border-radius: var(--radius-s);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .pill-act:hover {
    background: var(--surface-3);
  }
  .reply-bar {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .reply-bar input {
    flex: 1;
    min-width: 0;
    padding: 6px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border-strong);
    background: var(--surface-1);
    color: var(--text);
    font-size: 12.5px;
  }
  .n-tools {
    display: flex;
    gap: 2px;
    flex: none;
  }

  /* Controls Card */
  .toggles-row {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 7px;
    margin-bottom: 14px;
  }
  .t-circle {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 10px 4px;
    border-radius: var(--radius-m);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background var(--dur-1), color var(--dur-1), border-color var(--dur-1);
  }
  .t-circle:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }
  .t-circle.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .t-circle:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .speaker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding-top: 10px;
    margin-top: 8px;
    border-top: 1px solid var(--border);
  }
  .face-pill {
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-2);
    padding: 2px 8px;
    border-radius: var(--radius-xs);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .face-pill:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }
  .start-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    color: var(--text);
    padding: 5px 11px;
    border-radius: var(--radius-s);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .start-btn.rec-btn {
    background: var(--danger);
    border-color: transparent;
    color: #fff;
  }
  .sp-label {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }

  /* Camera Row */
  .cam-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .cam-card {
    display: grid;
    gap: 10px;
    text-align: left;
    cursor: pointer;
    padding: 13px 14px;
    border-radius: var(--radius-m);
    transition: border-color var(--dur-1), background var(--dur-1);
  }
  .cam-card:hover:not(:disabled) {
    border-color: var(--border-strong);
    background: var(--surface-2);
  }
  .cam-card.active {
    border-color: var(--accent);
  }
  .cam-card:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .cam-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  /* Type on phone */
  .type-card {
    display: grid;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius-m);
  }
  .type-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .type-left {
    display: inline-flex;
    align-items: center;
    gap: 9px;
    font-size: 13.5px;
  }
  .start-btn {
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    color: var(--text);
    padding: 5px 12px;
    border-radius: var(--radius-s);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .start-btn:hover:not(:disabled) {
    background: var(--surface-3);
  }
  .start-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .type-box {
    display: flex;
    gap: 6px;
  }
  .type-box input {
    flex: 1;
    min-width: 0;
    padding: 7px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    color: var(--text);
    font-size: 12.5px;
  }
  .key-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .kbtn {
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-2);
    padding: 3px 9px;
    border-radius: var(--radius-xs);
    font-size: 11.5px;
    cursor: pointer;
  }
  .kbtn:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  /* Timeline */
  .tl-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .tl-list li {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .tl-ico {
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--accent-text);
    display: grid;
    place-items: center;
  }
  .tl-body {
    min-width: 0;
    flex: 1;
  }
  .tl-title {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tl-sub {
    font-size: 11.5px;
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Shared small buttons */
  .ghost-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .ghost-btn:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }
  .ghost-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .link-btn {
    border: 0;
    background: none;
    color: var(--accent-text);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    padding: 2px 4px;
  }
  .link-btn:hover:not(:disabled) {
    text-decoration: underline;
  }
  .link-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .icon-btn {
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: var(--text-3);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--surface-3);
    color: var(--text);
  }
  .primary-sm {
    border: 0;
    background: var(--accent);
    color: var(--on-accent);
    padding: 6px 12px;
    border-radius: var(--radius-s);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .primary-sm:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .primary-sm:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .empty {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-3);
    line-height: 1.45;
  }
</style>
