import { describe, expect, it } from 'vitest';
import { dueText, noteFrom, startsLikeReminder } from './logic';

const base = Math.floor(new Date(2026, 10, 19, 14, 30, 0).getTime() / 1000);
const plus = (s: number) => base + s;

describe('dueText', () => {
  it('counts minutes and hours for soon', () => {
    expect(dueText(plus(5), base)).toBe('in under a minute');
    expect(dueText(plus(20 * 60), base)).toBe('in 20 min');
    expect(dueText(plus(2 * 3600), base)).toBe('in 2 h');
    expect(dueText(plus(2 * 3600 + 15 * 60), base)).toBe('in 2 h 15 min');
  });
  it('says now for a time that came', () => {
    expect(dueText(base - 5, base)).toBe('now');
  });
  it('names today and tomorrow', () => {
    const tonight = Math.floor(new Date(2026, 10, 19, 22, 0, 0).getTime() / 1000);
    const tomorrow = Math.floor(new Date(2026, 10, 20, 9, 0, 0).getTime() / 1000);
    expect(dueText(tonight, base, 'en-US')).toMatch(/^today at /);
    expect(dueText(tomorrow, base, 'en-US')).toMatch(/^tomorrow at /);
  });
  it('gives a date for later', () => {
    const later = Math.floor(new Date(2026, 10, 27, 9, 0, 0).getTime() / 1000);
    expect(dueText(later, base, 'en-US')).toMatch(/27/);
  });
});

describe('what the search box understands', () => {
  it('knows a reminder by its first word in three languages', () => {
    for (const t of ['remind me in 5 min', 'Reminder: x', 'me lembre em 5 min', 'lembrete amanhã', 'recuérdame mañana', 'recordatorio hoy']) {
      expect(startsLikeReminder(t)).toBe(true);
    }
    expect(startsLikeReminder('remote desktop')).toBe(false);
    expect(startsLikeReminder('open downloads')).toBe(false);
  });
  it('takes the note text after the word', () => {
    expect(noteFrom('note buy milk')).toBe('buy milk');
    expect(noteFrom('nota: comprar leite')).toBe('comprar leite');
    expect(noteFrom('anotar - ligar para a Ana')).toBe('ligar para a Ana');
    expect(noteFrom('notes')).toBeNull();
    expect(noteFrom('note')).toBeNull();
    expect(noteFrom('notepad')).toBeNull();
  });
});
