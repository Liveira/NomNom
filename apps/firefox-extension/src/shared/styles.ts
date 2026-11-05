/** One stylesheet for every site. Colours come from the site's own variables where it has them,
 *  so the button looks native in light and dark themes. */
export const CSS = `
.nomnom-wrap{display:inline-flex;align-items:center;position:relative;margin:0 4px;font-family:inherit;vertical-align:middle;--nn-accent:#ffb070}
.nomnom-wrap button{font:inherit;color:inherit;cursor:pointer;border:0;background:transparent;padding:0;display:inline-flex;align-items:center;gap:6px}
.nomnom-main{height:36px;padding:0 14px;border-radius:18px;font-family:Roboto,Arial,sans-serif;font-size:14px;line-height:20px;font-weight:500;white-space:nowrap;transition:background-color .12s,transform .12s}
.nomnom-main .nomnom-text{font-size:14px}
.nomnom-main:active{transform:scale(.97)}
.nomnom-main:focus-visible,.nomnom-more:focus-visible,.nomnom-menu button:focus-visible{outline:2px solid var(--nn-accent);outline-offset:2px}
.nomnom-main svg{width:18px;height:18px;flex:none}
.nomnom-more{width:22px;height:36px;justify-content:center;border-radius:0 18px 18px 0;opacity:.8}
.nomnom-more svg{width:12px;height:12px}
.nomnom-wrap[data-state=sending] .nomnom-main{opacity:.6;cursor:progress}
.nomnom-wrap[data-state=ok] .nomnom-main{color:#2ba640}
.nomnom-wrap[data-state=error] .nomnom-main{color:#e5484d}
.nomnom-menu{position:absolute;z-index:2147483000;top:calc(100% + 4px);right:0;min-width:180px;padding:6px;border-radius:12px;background:var(--yt-spec-menu-background,#fff);color:var(--yt-spec-text-primary,#0f0f0f);box-shadow:0 8px 28px rgba(0,0,0,.35);border:1px solid rgba(128,128,128,.25)}
@media (prefers-color-scheme:dark){.nomnom-menu{background:var(--yt-spec-menu-background,#282828);color:var(--yt-spec-text-primary,#f1f1f1)}}
.nomnom-menu[hidden]{display:none}
.nomnom-menu button{width:100%;padding:8px 10px;border-radius:8px;font-size:14px;text-align:left;justify-content:flex-start}
.nomnom-menu button:hover{background:rgba(128,128,128,.2)}
.nomnom-toast{position:absolute;z-index:2147483000;top:calc(100% + 4px);left:50%;transform:translateX(-50%);padding:6px 10px;border-radius:8px;font-size:13px;white-space:nowrap;background:#222;color:#fff;pointer-events:none}
.nomnom-wrap[data-site=youtube]{--nn-chip:var(--yt-spec-button-chip-background,var(--yt-spec-badge-chip-background,var(--yt-spec-additive-background,rgba(128,128,128,.2))));--nn-chip-hover:var(--yt-spec-button-chip-background-hover,var(--yt-spec-mono-tonal-hover,rgba(128,128,128,.32)))}
.nomnom-wrap[data-site=youtube] .nomnom-main{background:var(--nn-chip);color:var(--yt-spec-text-primary,currentColor);border-radius:18px 0 0 18px;padding:0 12px 0 14px}
.nomnom-wrap[data-site=youtube] .nomnom-main:hover,.nomnom-wrap[data-site=youtube] .nomnom-more:hover{background:var(--nn-chip-hover)}
.nomnom-wrap[data-site=youtube] .nomnom-more{background:var(--nn-chip);color:var(--yt-spec-text-primary,currentColor);border-left:1px solid rgba(128,128,128,.25)}
.nomnom-wrap[data-site=youtube][data-compact] .nomnom-main{width:48px;height:48px;padding:0;justify-content:center;border-radius:50%;background:rgba(0,0,0,.3);color:#fff}
.nomnom-wrap[data-compact]:not([data-site=youtube]) .nomnom-main{width:34px;height:34px;padding:0;justify-content:center;border-radius:50%}
.nomnom-wrap[data-compact]:not([data-site=youtube]) .nomnom-main:hover{background:rgba(255,176,112,.18)}
.nomnom-overlay{position:absolute;top:8px;right:8px;z-index:3}
`;

export function ensureStyles(doc: Document = document): void {
  if (doc.querySelector('style[data-nomnom-style]')) return;
  const style = doc.createElement('style');
  style.setAttribute('data-nomnom-style', '');
  style.textContent = CSS;
  (doc.head ?? doc.documentElement).appendChild(style);
}
