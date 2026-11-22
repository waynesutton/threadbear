// Agent-reported task progress is independent of the runner's loose-end checks.
import type { ThreadCompletion } from './protocol.ts';

export const MAX_COMPLETION_TASKS = 30;

type CompletionTask = ThreadCompletion['tasks'][number];

export function mergeCompletion(previous: ThreadCompletion | undefined, input: unknown): ThreadCompletion | undefined {
  if (!input || typeof input !== 'object') return previous;
  const value = input as Record<string, unknown>;
  if (!Array.isArray(value.tasks) || value.tasks.length > MAX_COMPLETION_TASKS) return previous;
  const updates: CompletionTask[] = [];
  const seen = new Set<string>();
  for (const raw of value.tasks) {
    if (!raw || typeof raw !== 'object') return previous;
    const t = raw as Record<string, unknown>;
    if (typeof t.id !== 'string' || !t.id.trim() || t.id.length > 64 || seen.has(t.id) ||
      typeof t.title !== 'string' || !t.title.trim() || t.title.length > 120 ||
      (t.status !== 'completed' && t.status !== 'pending' && t.status !== 'blocked')) return previous;
    seen.add(t.id);
    updates.push({ id: t.id, title: t.title.trim(), status: t.status as ThreadCompletion['tasks'][number]['status'] });
  }
  const removed = value.removeIds ?? [];
  if (!Array.isArray(removed) || removed.length > MAX_COMPLETION_TASKS || removed.some(id => typeof id !== 'string' || !id.trim() || id.length > 64 || seen.has(id))) return previous;
  const tasks = new Map((previous?.tasks ?? []).map(t => [t.id, { ...t }]));
  for (const id of removed) tasks.delete(id);
  for (const t of updates) tasks.set(t.id, t);
  // A long thread keeps adding requests, and dropping the whole update at 30
  // left its list quietly stuck. Finished work is the least useful to keep, so
  // the oldest completed tasks go first; only open work past the cap is refused.
  if (tasks.size > MAX_COMPLETION_TASKS) {
    const fresh = new Set(updates.map(t => t.id));
    for (const [id, t] of tasks) {
      if (tasks.size <= MAX_COMPLETION_TASKS) break;
      if (t.status === 'completed' && !fresh.has(id)) tasks.delete(id);
    }
    for (const [id, t] of tasks) {
      if (tasks.size <= MAX_COMPLETION_TASKS) break;
      if (t.status === 'completed') tasks.delete(id);
    }
    if (tasks.size > MAX_COMPLETION_TASKS) return previous;
  }
  return tasks.size ? { tasks: [...tasks.values()] } : undefined;
}

export function completionOpen(completion: ThreadCompletion | undefined): boolean {
  return completion?.tasks.some(t => t.status !== 'completed') ?? false;
}
