import { CATALOG } from './catalog';
import { DomTranslator } from './dom';
import { detectLang, type Lang } from './engine';

export type LangSetting = 'auto' | Lang;

export const LANGUAGES: { id: LangSetting; name: string; note: string }[] = [
  { id: 'auto', name: 'Automatic', note: 'Use the language of your computer' },
  { id: 'en', name: 'English', note: 'English' },
  { id: 'pt', name: 'Português', note: 'Português (Brasil)' },
  { id: 'es', name: 'Español', note: 'Español' },
];

class I18n {
  /** The language in use. Reading it in a component or an effect makes that code run again on a change. */
  lang = $state<Lang>('en');
  private dom: DomTranslator | null = null;

  /** Starts watching the page. Call once, after the page exists. */
  start(setting: LangSetting) {
    this.dom = new DomTranslator(CATALOG);
    this.apply(setting);
  }

  apply(setting: LangSetting) {
    const list = navigator.languages?.length ? navigator.languages : [navigator.language];
    const lang = setting === 'auto' ? detectLang(list) : setting;
    this.lang = lang;
    document.documentElement.lang = lang === 'pt' ? 'pt-BR' : lang;
    this.dom?.setLanguage(lang);
  }

  residual(): string[] {
    return this.dom ? this.dom.residual() : [];
  }

  /** For text built in code. Text written in templates is translated on screen without this. */
  t(text: string): string {
    void this.lang;
    return this.dom ? this.dom.tr(text) : text;
  }
}

export const i18n = new I18n();

// The test scripts read this to list any text that still has no translation.
if (import.meta.env.DEV) (window as unknown as { __i18nResidual: () => string[] }).__i18nResidual = () => i18n.residual();
