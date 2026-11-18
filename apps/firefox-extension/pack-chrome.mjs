// Zips dist-chrome/ into ../../dist/extension/nomnom-chrome-extension.zip (Chrome, Edge and other
// Chromium browsers load it as an unpacked extension).
import { mkdirSync, rmSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';

const out = resolve('../../dist/extension');
mkdirSync(out, { recursive: true });
const zip = resolve(out, 'nomnom-chrome-extension.zip');
rmSync(zip, { force: true });
if (process.platform === 'win32') {
  execFileSync('powershell', ['-NoProfile', '-Command', `Compress-Archive -Path '${resolve('dist-chrome')}\*' -DestinationPath '${zip}'`], { stdio: 'inherit' });
} else {
  execFileSync('zip', ['-r', '-q', zip, '.'], { cwd: resolve('dist-chrome'), stdio: 'inherit' });
}
console.log('wrote', zip);
