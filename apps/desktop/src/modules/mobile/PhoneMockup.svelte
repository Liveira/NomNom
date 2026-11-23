<script lang="ts">
  let {
    level = 0,
    charging = false,
    connected = false,
    transport = 'wifi',
  }: {
    level?: number;
    charging?: boolean;
    connected?: boolean;
    transport?: string;
  } = $props();

  const clamped = $derived(Math.max(0, Math.min(100, Math.round(level))));
  const R = 22;
  const C = 2 * Math.PI * R;
  const offset = $derived(C * (1 - clamped / 100));
  const ringColor = $derived(
    !connected
      ? 'var(--text-3)'
      : charging
        ? '#45d483'
        : clamped <= 20
          ? 'var(--danger)'
          : 'var(--accent)',
  );
</script>

<div class="phone-frame" class:offline={!connected} aria-hidden="true">
  <div class="screen">
    <div class="top-bar">
      <span class="sig">{connected ? (transport === 'wifi' ? 'Wi-Fi' : 'USB') : '—'}</span>
      <span class="notch"></span>
      <span class="dot"></span>
    </div>

    <div class="ring-wrap">
      <svg viewBox="0 0 56 56">
        <circle class="track" cx="28" cy="28" r={R} />
        <circle
          class="fill"
          cx="28"
          cy="28"
          r={R}
          stroke={ringColor}
          stroke-dasharray={C}
          stroke-dashoffset={connected ? offset : C}
        />
      </svg>
      <span class="pct tabular">{connected ? `${clamped}%` : '—'}</span>
    </div>

    <span class="caption">{!connected ? 'Offline' : charging ? 'Charging' : 'Battery'}</span>
  </div>
</div>

<style>
  .phone-frame {
    width: 78px;
    height: 142px;
    flex: none;
    border-radius: 16px;
    padding: 4px;
    background: var(--surface-3);
    border: 1.5px solid var(--border-strong);
    box-shadow: var(--shadow-card);
  }
  .phone-frame.offline {
    opacity: 0.6;
  }
  .screen {
    width: 100%;
    height: 100%;
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent) 26%, var(--surface-1));
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    padding: 7px 6px 12px;
    box-sizing: border-box;
  }
  .top-bar {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 8px;
    font-weight: 700;
    color: var(--text-2);
    letter-spacing: 0.02em;
  }
  .notch {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--bg);
    border: 1px solid var(--border-strong);
  }
  .dot {
    width: 10px;
    height: 4px;
    border-radius: 2px;
    background: var(--text-3);
    opacity: 0.6;
  }
  .ring-wrap {
    position: relative;
    width: 52px;
    height: 52px;
    display: grid;
    place-items: center;
  }
  svg {
    position: absolute;
    inset: 0;
    transform: rotate(-90deg);
  }
  circle {
    fill: none;
    stroke-width: 4;
  }
  .track {
    stroke: var(--surface-3);
    opacity: 0.65;
  }
  .fill {
    stroke-linecap: round;
    transition: stroke-dashoffset var(--dur-3) var(--ease-out);
  }
  .pct {
    font-size: 11px;
    font-weight: 700;
    color: var(--text);
  }
  .caption {
    font-size: 9px;
    font-weight: 600;
    color: var(--text-2);
  }
</style>
