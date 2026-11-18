import { buildCatalog, translateText, type Catalog, type Entry, type Lang } from './engine';

/**
 * Translates what is on screen. A MutationObserver watches the page, so text the interface writes
 * later (a progress line, a list that scrolls in) is translated too. Every text node and attribute
 * remembers its English source, so switching language, even back to English, always starts from it.
 * Anything inside translate="no" (file names, app names, paths) is left exactly as it is.
 */
const ATTRS = ['placeholder', 'title', 'aria-label', 'alt'];
const SKIP_TAGS = new Set(['SCRIPT', 'STYLE', 'TEXTAREA', 'CODE', 'KBD', 'SVG', 'PATH', 'CANVAS']);

interface Mem {
  src: string;
  out: string;
}

export class DomTranslator {
  private cat: Catalog = { exact: new Map(), patterns: [] };
  private memo = new Map<string, string>();
  private texts = new WeakMap<Node, Mem>();
  private attrs = new WeakMap<Element, Map<string, Mem>>();
  private observer: MutationObserver | null = null;

  constructor(
    private entries: Entry[],
    private root: HTMLElement = document.body,
  ) {}

  /** Translates a string from code (not from the page). */
  tr(text: string): string {
    return translateText(this.cat, text, this.memo);
  }

  setLanguage(lang: Lang) {
    this.cat = buildCatalog(this.entries, lang);
    this.memo.clear();
    this.walk(this.root);
    if (lang === 'en') {
      // English is the text the interface is written in: nothing will need translating later.
      this.observer?.disconnect();
      this.observer = null;
      return;
    }
    if (!this.observer) {
      this.observer = new MutationObserver((records) => this.onMutations(records));
      this.observer.observe(this.root, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ATTRS });
    }
  }

  /** Text on screen that was not translated (for checking the catalog; dev tools only). */
  residual(): string[] {
    const out = new Set<string>();
    const seen = (src: string, shown: string) => {
      if (src === shown && /[A-Za-z]{3,}/.test(src)) out.add(src.replace(/\s+/g, ' ').trim());
    };
    const walk = (node: Node) => {
      if (node.nodeType === Node.TEXT_NODE) {
        if (this.skipped(node)) return;
        const m = this.texts.get(node);
        if (m && (node.parentElement?.offsetParent !== null || node.parentElement === document.body)) seen(m.src, m.out);
        return;
      }
      if (!(node instanceof Element) || SKIP_TAGS.has(node.tagName.toUpperCase()) || node.getAttribute('translate') === 'no') return;
      const am = this.attrs.get(node);
      if (am) for (const m of am.values()) seen(m.src, m.out);
      for (let c = node.firstChild; c; c = c.nextSibling) walk(c);
    };
    walk(this.root);
    return [...out];
  }

  private skipped(node: Node): boolean {
    for (let el: Element | null = node instanceof Element ? node : node.parentElement; el; el = el.parentElement) {
      if (SKIP_TAGS.has(el.tagName.toUpperCase())) return true;
      if (el.getAttribute('translate') === 'no') return true;
    }
    return false;
  }

  private text(node: Node) {
    if (this.skipped(node)) return;
    const now = node.nodeValue ?? '';
    const mem = this.texts.get(node);
    // Our own write comes back as a mutation; anything else is new English from the interface.
    if (mem && mem.out === now) {
      const out = translateText(this.cat, mem.src, this.memo);
      if (out !== mem.out) {
        mem.out = out;
        node.nodeValue = out;
      }
      return;
    }
    const out = translateText(this.cat, now, this.memo);
    this.texts.set(node, { src: now, out });
    if (out !== now) node.nodeValue = out;
  }

  private attr(el: Element, name: string) {
    if (this.skipped(el)) return;
    const now = el.getAttribute(name);
    if (now === null) return;
    let map = this.attrs.get(el);
    if (!map) this.attrs.set(el, (map = new Map()));
    const mem = map.get(name);
    if (mem && mem.out === now) {
      const out = translateText(this.cat, mem.src, this.memo);
      if (out !== mem.out) {
        mem.out = out;
        el.setAttribute(name, out);
      }
      return;
    }
    const out = translateText(this.cat, now, this.memo);
    map.set(name, { src: now, out });
    if (out !== now) el.setAttribute(name, out);
  }

  private walk(node: Node) {
    if (node.nodeType === Node.TEXT_NODE) {
      this.text(node);
      return;
    }
    if (!(node instanceof Element)) return;
    if (SKIP_TAGS.has(node.tagName.toUpperCase()) || node.getAttribute('translate') === 'no') return;
    for (const a of ATTRS) if (node.hasAttribute(a)) this.attr(node, a);
    for (let c = node.firstChild; c; c = c.nextSibling) this.walk(c);
  }

  private onMutations(records: MutationRecord[]) {
    for (const r of records) {
      if (r.type === 'characterData') this.text(r.target);
      else if (r.type === 'attributes') this.attr(r.target as Element, r.attributeName!);
      else for (const n of r.addedNodes) this.walk(n);
    }
  }
}
