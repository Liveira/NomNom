// Bundles the extension as plain, readable JavaScript (no minifying, so reviewers can read what
// ships). Two builds from the same source: dist/ for Firefox and dist-chrome/ for Chrome and Edge.
import { build } from 'esbuild';
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const entries = {
  background: 'src/background.ts',
  options: 'src/options.ts',
  'content-youtube': 'src/entries/youtube.ts',
  'content-x': 'src/entries/x.ts',
  'content-instagram': 'src/entries/instagram.ts',
  'content-reddit': 'src/entries/reddit.ts',
  'content-generic': 'src/entries/generic.ts',
};

/** The Chrome extension's ID, worked out from its public key the way Chrome does. The NomNom app
 *  needs the same ID to allow the extension to talk to it. */
export function chromeId(publicKeyBase64) {
  const digest = createHash('sha256').update(Buffer.from(publicKeyBase64, 'base64')).digest().subarray(0, 16);
  return [...digest].map((b) => String.fromCharCode(97 + (b >> 4)) + String.fromCharCode(97 + (b & 15))).join('');
}

async function bundle(outdir, target, banner) {
  rmSync(outdir, { recursive: true, force: true });
  mkdirSync(outdir, { recursive: true });
  await build({
    entryPoints: entries,
    outdir,
    bundle: true,
    format: 'iife',
    target,
    minify: false,
    sourcemap: false,
    legalComments: 'none',
    logLevel: 'info',
    ...(banner ? { banner: { js: banner } } : {}),
  });
  cpSync('src/options.html', `${outdir}/options.html`);
  cpSync('icons', `${outdir}/icons`, { recursive: true });
}

const firefox = JSON.parse(readFileSync('manifest.json', 'utf8'));

await bundle('dist', 'firefox128');
writeFileSync('dist/manifest.json', JSON.stringify(firefox, null, 2) + '\n');

// Chrome has no `browser` object; its `chrome` object does the same here, with promises.
await bundle('dist-chrome', 'chrome114', "if (typeof browser === 'undefined') globalThis.browser = globalThis.chrome;");
const chrome = structuredClone(firefox);
delete chrome.browser_specific_settings;
chrome.background = { service_worker: 'background.js' };
chrome.minimum_chrome_version = '114';
chrome.key = readFileSync('chrome/public-key.txt', 'utf8').trim();
writeFileSync('dist-chrome/manifest.json', JSON.stringify(chrome, null, 2) + '\n');
writeFileSync('dist-chrome/extension-id.txt', chromeId(chrome.key) + '\n');
