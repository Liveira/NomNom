import { describe, expect, it } from 'vitest';
import type { OrgBatch, OrgMove, OrgSkip } from '../../lib/ipc';
import { batchStatus, batchTitle, canUndo, fileName, groupMoves, groupSkips, headline, resultLine, undoLine } from './summary';

const move = (category: string, name: string): OrgMove => ({ src: `C:\\D\\${name}`, dst: `C:\\F\\${category}\\${name}`, category, cross_volume: false });
const skip = (code: OrgSkip['code'], name: string): OrgSkip => ({ path: `C:\\D\\${name}`, is_dir: false, code, message: 'x' });

describe('headline', () => {
  it('counts what is ready and what is left', () => {
    expect(headline({ total_moves: 12, total_skips: 3 })).toBe('12 files ready · 3 left alone');
    expect(headline({ total_moves: 1, total_skips: 0 })).toBe('1 file ready · 0 left alone');
  });
  it('says so plainly when nothing can move', () => {
    expect(headline({ total_moves: 0, total_skips: 0 })).toBe('Nothing here to organize.');
    expect(headline({ total_moves: 0, total_skips: 1 })).toBe('Nothing is ready to move. 1 item was left alone.');
    expect(headline({ total_moves: 0, total_skips: 4 })).toBe('Nothing is ready to move. 4 items were left alone.');
  });
});

describe('grouping', () => {
  it('groups moves by category in alphabetical order', () => {
    const g = groupMoves([move('Videos', 'a.mp4'), move('Audio', 'b.mp3'), move('Videos', 'c.mkv')]);
    expect(g.map((x) => [x.category, x.items.length])).toEqual([['Audio', 1], ['Videos', 2]]);
  });
  it('lists the reasons that matter for safety first', () => {
    const g = groupSkips([skip('unknown', 'a'), skip('conflict', 'b'), skip('software_folder', 'c'), skip('program', 'd')]);
    expect(g.map((x) => x.code)).toEqual(['software_folder', 'program', 'conflict', 'unknown']);
    expect(g[0].title).toBe('Folders that look like software');
  });
  it('every reason has a plain-English title', () => {
    const codes = ['link', 'hidden', 'cloud', 'in_progress', 'recently_changed', 'locked', 'unknown', 'mismatch', 'program', 'archive', 'empty', 'software_folder', 'folder_not_opened', 'protected', 'conflict', 'cross_volume', 'already_organized', 'unreadable', 'excluded', 'too_many'] as const;
    for (const c of codes) {
      const [g] = groupSkips([skip(c, 'x')]);
      expect(g.title.length, c).toBeGreaterThan(3);
      expect(g.title, c).not.toBe('Other');
    }
  });
});

describe('results', () => {
  it('describes a run and an undo honestly', () => {
    expect(resultLine({ moved: 3, failed: [], stopped: null })).toBe('Moved 3 files.');
    expect(resultLine({ moved: 1, failed: [{}], stopped: null })).toBe('Moved 1 file. 1 file was left where it was.');
    expect(resultLine({ moved: 2, failed: [{}, {}], stopped: 'The disk is full.' })).toBe('Moved 2 files. 2 files were left where they were. The disk is full.');
    expect(undoLine({ restored: 4, skipped: [] })).toBe('Put back 4 files.');
    expect(undoLine({ restored: 3, skipped: [{}] })).toBe('Put back 3 files. 1 file was left where it is now.');
  });
});

describe('history rows', () => {
  const b = (o: Partial<OrgBatch>): OrgBatch => ({ id: '1', mode: 'manual', source_root: '', dest_root: '', created_at: 0, state: 'done', moved: 5, undone: 0, failed: 0, ...o });
  it('says what happened and whether it can still be undone', () => {
    expect(batchTitle(b({}))).toBe('Manual · 5 files');
    expect(batchTitle(b({ mode: 'auto', moved: 1 }))).toBe('Automatic · 1 file');
    expect(batchStatus(b({}))).toBe('Done');
    expect(batchStatus(b({ moved: 0, undone: 5 }))).toBe('Put back');
    expect(batchStatus(b({ moved: 2, undone: 3 }))).toBe('3 put back, 2 still moved');
    expect(batchStatus(b({ failed: 1 }))).toBe('1 left where it was');
    expect(canUndo(b({}))).toBe(true);
    expect(canUndo(b({ moved: 0, undone: 5 }))).toBe(false);
  });
  it('takes the name from a Windows path', () => {
    expect(fileName('C:\\Users\\me\\Downloads\\clip.mp4')).toBe('clip.mp4');
    expect(fileName('C:/a/b/')).toBe('b');
  });
});
