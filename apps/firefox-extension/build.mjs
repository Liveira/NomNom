// Bundles the extension into dist/ as plain, readable JavaScript (no minifying, so reviewers can
// read what ships).
import { build } from 'esbuild';
import { cpSync, mkdirSync, rmSync } from 'node:fs';

rmSync('dist', { recursive: true, force: true });
mkdirSync('dist', { recursive: true });

const entries = {
  background: 'src/background.ts',
  options: 'src/options.ts',
  'content-youtube': 'src/entries/youtube.ts',
  'content-x': 'src/entries/x.ts',
  'content-instagram': 'src/entries/instagram.ts',
  'content-reddit': 'src/entries/reddit.ts',
};

await build({
  entryPoints: entries,
  outdir: 'dist',
  bundle: true,
  format: 'iife',
  target: 'firefox128',
  minify: false,
  sourcemap: false,
  legalComments: 'none',
  logLevel: 'info',
});

cpSync('manifest.json', 'dist/manifest.json');
cpSync('src/options.html', 'dist/options.html');
cpSync('icons', 'dist/icons', { recursive: true });
