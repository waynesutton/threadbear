export type CanonicalStatus = 'backlog' | 'todo' | 'doing' | 'done';
export type TaskColumn = { id: string; name: string; status: CanonicalStatus };
// Only for a backend before API 89 with nothing saved: from 89 the backend
// always sends every column, so one that was deleted stays gone.
export const defaultColumns: TaskColumn[] = [
  { id: 'backlog', name: 'Backlog', status: 'backlog' },
  { id: 'todo', name: 'Todo', status: 'todo' },
  { id: 'doing', name: 'Doing', status: 'doing' },
  { id: 'done', name: 'Done', status: 'done' },
];
export function columnDefinitions(saved?: TaskColumn[]): TaskColumn[] {
  return saved?.length ? [...saved] : defaultColumns;
}
export function orderedColumns(columns: TaskColumn[], saved?: string[]): string[] {
  const ids = columns.map(c => c.id);
  return [...new Set([...(saved ?? []).filter(id => ids.includes(id)), ...ids])];
}
export function taskColumn(task: { columnId?: string; status: CanonicalStatus }, columns: TaskColumn[]): string {
  if (columns.some(c => c.id === task.columnId && c.status === task.status)) return task.columnId!;
  if (columns.some(c => c.id === task.status)) return task.status;
  return columns.find(c => c.status === task.status)?.id ?? 'todo';
}
// Where tasks:deleteColumn sends a column's tasks: its status's built-in
// column, else the first left with that status, else Building (id todo).
export function deleteTarget(id: string, columns: TaskColumn[], order: string[]): string {
  const status = columns.find(c => c.id === id)?.status ?? 'todo';
  const left = order.filter(c => c !== id && columns.some(d => d.id === c));
  if (status !== id && left.includes(status)) return status;
  return left.find(c => columns.find(d => d.id === c)?.status === status) ?? 'todo';
}
