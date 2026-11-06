/// <reference types="node" />
/** Everything the app says to the user is plain, direct English. No marketing phrases. */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const BANNED = [
  'ultimate', 'seamless', 'empower', 'unlock', 'revolution', 'all-in-one', 'cutting-edge',
  'supercharge', 'game-changing', 'game changing', 'effortless', 'next-level', 'next level',
  'blazing', 'magical', 'magic ', 'best-in-class', 'world-class', 'state-of-the-art', 'delight',
  'leverage', 'robust', 'streamline', 'turbocharge', 'incredible', 'amazing', 'awesome',
];
const BANNED_RE = new RegExp(`(?:\\b(?:${BANNED.map((b) => b.replace(/[-\s]/g, '[-\\s]')).join('|')})|\\w!!)`, 'i');

function walk(dir: string, exts: string[], out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    if (name === 'node_modules' || name === 'dist' || name === 'target' || name === 'gen') continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, exts, out);
    else if (exts.some((e) => name.endsWith(e)) && !name.endsWith('.test.ts')) out.push(p);
  }
  return out;
}

const root = resolve(process.cwd());

/** Quoted strings only, so identifiers and comments in code don't trip the check. */
function rustStrings(text: string): string[] {
  return [...text.matchAll(/"((?:[^"\\\n]|\\.)*)"/g)].map((m) => m[1]);
}

describe('user-facing text', () => {
  it('has no marketing phrases in the interface', () => {
    const hits: string[] = [];
    for (const file of walk(join(root, 'src'), ['.svelte', '.ts', '.html'])) {
      const text = readFileSync(file, 'utf8');
      for (const [i, line] of text.split('\n').entries()) {
        if (BANNED_RE.test(line)) hits.push(`${relative(root, file)}:${i + 1}: ${line.trim()}`);
      }
    }
    for (const file of walk(root, ['.html']).filter((f) => !f.includes('node_modules'))) {
      const text = readFileSync(file, 'utf8');
      if (BANNED_RE.test(text)) hits.push(relative(root, file));
    }
    expect(hits).toEqual([]);
  });

  it('has no marketing phrases in messages written by the app', () => {
    const hits: string[] = [];
    for (const file of walk(join(root, 'src-tauri', 'src'), ['.rs'])) {
      for (const s of rustStrings(readFileSync(file, 'utf8'))) {
        if (BANNED_RE.test(s)) hits.push(`${relative(root, file)}: "${s}"`);
      }
    }
    for (const dir of ['../../crates']) {
      for (const file of walk(resolve(root, dir), ['.rs'])) {
        for (const s of rustStrings(readFileSync(file, 'utf8'))) {
          if (BANNED_RE.test(s)) hits.push(`${relative(root, file)}: "${s}"`);
        }
      }
    }
    expect(hits).toEqual([]);
  });

  it('writes the product name the same way everywhere a person reads it', () => {
    // "NomNom" in running text, NOMNOM only for the exe, the data folder and the logo wordmark.
    const odd: string[] = [];
    for (const file of walk(join(root, 'src'), ['.svelte'])) {
      const text = readFileSync(file, 'utf8');
      const visible = text.replace(/<style[\s\S]*?<\/style>/g, '').replace(/<script[\s\S]*?<\/script>/g, '');
      for (const m of visible.matchAll(/>([^<>{}]*\bNOMNOM\b[^<>{}]*)</g)) odd.push(`${relative(root, file)}: ${m[1].trim()}`);
    }
    expect(odd).toEqual([]);
  });
});
