// Ordering rules shared by the list and the board, kept apart so they're tested.
export type Status = 'backlog' | 'todo' | 'doing' | 'done';
export const STATUSES: Status[] = ['backlog', 'todo', 'doing', 'done'];

/** The saved column order with any status it's missing put back (Doing after Todo). */
export const LIST_STATUSES: Status[] = ['doing', 'todo', 'backlog', 'done'];
export function fullOrder(saved: readonly Status[] | undefined, fallback: readonly Status[] = STATUSES): Status[] {
  const out = (saved ?? fallback).filter((s, i, all) => STATUSES.includes(s) && all.indexOf(s) === i);
  for (const s of STATUSES) if (!out.includes(s)) out.splice(s === 'doing' ? out.indexOf('todo') + 1 : out.length, 0, s);
  return out;
}

/** Project groups to draw: the saved order first, then new ones, with No project last. */
export function groupOrder(saved: readonly string[] | undefined, keys: string[], filter = ''): string[] {
  const present = [...new Set(filter ? [filter] : keys)];
  const known = (saved ?? []).filter((k) => present.includes(k));
  const rest = present.filter((k) => !known.includes(k)).sort((a, b) => (a === 'none' ? 1 : b === 'none' ? -1 : 0));
  return [...known, ...rest];
}

/** A position that sorts just before `beforeId` among `peers` (sorted), or after the last. */
export function positionAt(peers: { id: string; position: number }[], beforeId?: string): number {
  const index = beforeId ? peers.findIndex((t) => t.id === beforeId) : -1;
  const at = index < 0 ? peers.length : index;
  const high = peers[at]?.position;
  const low = peers[at - 1]?.position;
  if (high === undefined) return (low ?? Date.now()) + 1000;
  if (low === undefined) return high - 1000;
  return (low + high) / 2;
}

/** What's typed in the composer: the first line is the title, anything after is the note. */
export function splitDraft(text: string): { title: string; notes: string } {
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const first = lines.findIndex((l) => l.trim());
  if (first < 0) return { title: '', notes: '' };
  return { title: lines[first].trim().slice(0, 500), notes: lines.slice(first + 1).join('\n').trim() };
}

/** Moves `item` to where `target` is: dragged right or down, it lands after it; left or up, before. */
export function moveTo<T>(list: readonly T[], item: T, target: T): T[] {
  const from = list.indexOf(item);
  const to = list.indexOf(target);
  if (from < 0 || to < 0 || from === to) return [...list];
  const next = list.filter((x) => x !== item);
  next.splice(to, 0, item);
  return next;
}
