/** What the command palette and the launcher need: matching what was typed, and a small calculator. */

/**
 * How well `text` answers `query`: 0 means not at all. Every word of the query has to match.
 * A start of the text beats a start of a word, which beats the middle, which beats scattered letters.
 */
export function score(query: string, text: string): number {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return 1;
  const t = text.toLowerCase();
  let total = 0;
  for (const w of words) {
    const s = wordScore(w, t);
    if (s === 0) return 0;
    total += s;
  }
  return total;
}

function wordScore(w: string, t: string): number {
  if (t === w) return 1000;
  if (t.startsWith(w)) return 800 - Math.min(t.length - w.length, 200);
  const at = t.indexOf(w);
  if (at >= 0) {
    const wordStart = /[\s\-_.\\/]/.test(t[at - 1] ?? ' ');
    return (wordStart ? 600 : 400) - Math.min(at, 200);
  }
  // The letters in order, not next to each other: "vsc" finds "Visual Studio Code".
  if (w.length < 2) return 0;
  let from = 0;
  let gaps = 0;
  let last = -1;
  for (const ch of w) {
    const i = t.indexOf(ch, from);
    if (i < 0) return 0;
    if (last >= 0 && i !== last + 1) gaps += 1;
    last = i;
    from = i + 1;
  }
  return Math.max(10, 120 - gaps * 12 - Math.min(t.length, 60));
}

export interface Rankable {
  title: string;
  /** More words that should find it, such as "colour" for "Theme". */
  keywords?: string;
}

/** The best matches first. Ties keep the order they came in. */
export function rank<T extends Rankable>(items: T[], query: string, limit = 40): T[] {
  if (!query.trim()) return items.slice(0, limit);
  const scored: { item: T; s: number; i: number }[] = [];
  items.forEach((item, i) => {
    const a = score(query, item.title);
    const b = item.keywords ? score(query, item.keywords) * 0.5 : 0;
    const s = Math.max(a, b);
    if (s > 0) scored.push({ item, s, i });
  });
  scored.sort((x, y) => y.s - x.s || x.i - y.i);
  return scored.slice(0, limit).map((x) => x.item);
}

// ---- A calculator that never runs the text as code. ----

type Tok = { t: 'n'; v: number } | { t: 'op'; v: string } | { t: 'id'; v: string };

function tokenize(src: string): Tok[] | null {
  const out: Tok[] = [];
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    if (/\s/.test(c)) {
      i++;
    } else if (/[0-9.]/.test(c)) {
      let j = i;
      while (j < src.length && /[0-9.,]/.test(src[j])) j++;
      const text = src.slice(i, j).replace(/,/g, '.');
      // "1.000,5" is not supported: one separator at most, so a typo never becomes a number.
      if ((text.match(/\./g) ?? []).length > 1 || text === '.') return null;
      out.push({ t: 'n', v: Number(text) });
      i = j;
    } else if (/[a-z]/i.test(c)) {
      let j = i;
      while (j < src.length && /[a-z0-9_]/i.test(src[j])) j++;
      out.push({ t: 'id', v: src.slice(i, j).toLowerCase() });
      i = j;
    } else if ('+-*/^%()×÷'.includes(c)) {
      out.push({ t: 'op', v: c === '×' ? '*' : c === '÷' ? '/' : c });
      i++;
    } else {
      return null;
    }
  }
  return out;
}

const FUNCTIONS: Record<string, (x: number) => number> = {
  sqrt: Math.sqrt,
  abs: Math.abs,
  sin: Math.sin,
  cos: Math.cos,
  tan: Math.tan,
  ln: Math.log,
  log: Math.log10,
  round: Math.round,
  floor: Math.floor,
  ceil: Math.ceil,
};
const CONSTANTS: Record<string, number> = { pi: Math.PI, e: Math.E };

/**
 * The value of an arithmetic expression, or `null` when the text isn't one. A bare number is not
 * "a calculation", so typing `7` doesn't offer an answer.
 */
export function calc(src: string): number | null {
  const toks = tokenize(src.trim());
  if (!toks || toks.length === 0) return null;
  const interesting = toks.some((t) => (t.t === 'op' && !'()'.includes(t.v)) || t.t === 'id');
  if (!interesting) return null;
  let pos = 0;
  const peek = () => toks[pos];
  const isOp = (v: string) => peek()?.t === 'op' && peek().v === v;

  const parseExpr = (): number => {
    let x = parseTerm();
    while (isOp('+') || isOp('-')) {
      const op = (toks[pos++] as { v: string }).v;
      const y = parseTerm();
      x = op === '+' ? x + y : x - y;
    }
    return x;
  };
  const parseTerm = (): number => {
    let x = parsePower();
    while (isOp('*') || isOp('/') || isOp('%')) {
      // A % with nothing after it means "percent"; with a number after it, "remainder".
      const next = toks[pos + 1];
      if (isOp('%') && (!next || (next.t === 'op' && next.v !== '(' && next.v !== '-'))) break;
      const op = (toks[pos++] as { v: string }).v;
      const y = parsePower();
      x = op === '*' ? x * y : op === '/' ? x / y : x % y;
    }
    return x;
  };
  const parsePower = (): number => {
    const base = parseUnary();
    if (isOp('^')) {
      pos++;
      return Math.pow(base, parsePower());
    }
    return base;
  };
  const parseUnary = (): number => {
    if (isOp('-')) {
      pos++;
      return -parseUnary();
    }
    if (isOp('+')) {
      pos++;
      return parseUnary();
    }
    return parsePostfix();
  };
  const parsePostfix = (): number => {
    let x = parseAtom();
    while (isOp('%') && (!toks[pos + 1] || (toks[pos + 1].t === 'op' && toks[pos + 1].v !== '(' && toks[pos + 1].v !== '-'))) {
      pos++;
      x /= 100;
    }
    return x;
  };
  const parseAtom = (): number => {
    const t = toks[pos++];
    if (!t) throw new Error('end');
    if (t.t === 'n') return t.v;
    if (t.t === 'id') {
      if (t.v in CONSTANTS) return CONSTANTS[t.v];
      const f = FUNCTIONS[t.v];
      if (!f) throw new Error('unknown');
      if (!isOp('(')) throw new Error('call');
      pos++;
      const arg = parseExpr();
      if (!isOp(')')) throw new Error('close');
      pos++;
      return f(arg);
    }
    if (t.v === '(') {
      const x = parseExpr();
      if (!isOp(')')) throw new Error('close');
      pos++;
      return x;
    }
    throw new Error('unexpected');
  };

  try {
    const v = parseExpr();
    if (pos !== toks.length) return null;
    return Number.isFinite(v) ? v : null;
  } catch {
    return null;
  }
}

/** A result for people: no float dust (0.1 + 0.2), no scientific notation for ordinary sizes. */
export function showNumber(n: number): string {
  if (Number.isInteger(n) && Math.abs(n) < 1e15) return String(n);
  const s = Number(n.toPrecision(12));
  if (Math.abs(s) >= 1e15 || (Math.abs(s) < 1e-6 && s !== 0)) return s.toExponential(6).replace(/\.?0+e/, 'e');
  return String(s);
}
