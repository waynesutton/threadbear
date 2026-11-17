export type CanonicalStatus = 'backlog' | 'todo' | 'doing' | 'done';
export type TaskColumn = { id: string; name: string; status: CanonicalStatus };
export const defaultColumns: TaskColumn[] = [
  { id: 'backlog', name: 'Backlog', status: 'backlog' },
  { id: 'todo', name: 'Todo', status: 'todo' },
  { id: 'doing', name: 'Doing', status: 'doing' },
  { id: 'done', name: 'Done', status: 'done' },
];
export function columnDefinitions(saved?: TaskColumn[]): TaskColumn[] {
  const result = saved ? [...saved] : [];
  for (const column of defaultColumns) if (!result.some(c => c.id === column.id)) result.push(column);
  return result;
}
export function orderedColumns(columns: TaskColumn[], saved?: string[]): string[] {
  const ids = columns.map(c => c.id);
  return [...new Set([...(saved ?? []).filter(id => ids.includes(id)), ...ids])];
}
export function taskColumn(task: { columnId?: string; status: CanonicalStatus }, columns: TaskColumn[]): string {
  return columns.some(c => c.id === task.columnId && c.status === task.status) ? task.columnId! : task.status;
}
