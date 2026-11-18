<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import Logo from '../../lib/icons/Logo.svelte';
  import { updateIpc } from '../../lib/ipc';

  let version = $state('');
  void updateIpc.version().then((v) => (version = v));

  const credits = [
    { name: 'Lucide', what: 'icons, and the shapes in the app icon', licence: 'ISC', href: 'https://lucide.dev' },
    { name: 'Fredoka', what: 'headings', licence: 'SIL OFL 1.1', href: 'https://fonts.google.com/specimen/Fredoka' },
    { name: 'Nunito', what: 'text', licence: 'SIL OFL 1.1', href: 'https://fonts.google.com/specimen/Nunito' },
    { name: 'yt-dlp and FFmpeg', what: 'downloads and conversions (fetched on request)', licence: 'Unlicense, GPL', href: 'https://github.com/yt-dlp/yt-dlp' },
  ];
</script>

<section class="group" aria-labelledby="about">
  <h3 id="about">NomNom</h3>
  <div class="who">
    <Logo size={52} />
    <div>
      <div class="ver">Version {version || '…'}</div>
      <div class="hint">Everything runs on this computer. No account, no tracking.</div>
      <a class="link" href="https://github.com/Liveira/NomNom" target="_blank" rel="noreferrer"><Icon name="external" size={13} /> Source code and releases</a>
    </div>
  </div>
</section>

<section class="group" aria-labelledby="oss">
  <h3 id="oss">Open source</h3>
  <p class="hint lead">NomNom is built on these open source projects.</p>
  <ul class="credits">
    {#each credits as c (c.name)}
      <li>
        <a href={c.href} target="_blank" rel="noreferrer">{c.name}</a>
        <span class="what">{c.what}</span>
        <span class="lic">{c.licence}</span>
      </li>
    {/each}
  </ul>
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .who { display: flex; align-items: center; gap: 16px; }
  .ver { font-family: var(--font-display); font-weight: 700; font-size: 18px; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .lead { margin: 0 0 10px; }
  .link { display: inline-flex; align-items: center; gap: 5px; margin-top: 6px; color: var(--accent-text); }
  .credits { list-style: none; margin: 0; padding: 0; display: grid; }
  .credits li { display: grid; grid-template-columns: 150px 1fr auto; gap: 12px; align-items: baseline; padding: 9px 0; border-top: 1px solid var(--border); }
  .credits li:first-child { border-top: 0; }
  .credits a { color: var(--accent-text); font-weight: 700; }
  .what { color: var(--text-2); font-size: 12.5px; }
  .lic { color: var(--text-3); font-size: 12px; white-space: nowrap; }
</style>
