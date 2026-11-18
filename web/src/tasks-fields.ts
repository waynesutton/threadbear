// Tasks' issue fields (backend API 87): priority names, due dates as calendar
// days in the browser's own time zone, and the due picker's month grid.
export type Priority = 'urgent' | 'high' | 'normal' | 'low';
export const PRIORITIES: { id: Priority | null; name: string }[] = [
  { id: null, name: 'No priority' },
  { id: 'urgent', name: 'Urgent' },
  { id: 'high', name: 'High' },
  { id: 'normal', name: 'Normal' },
  { id: 'low', name: 'Low' },
];
export const priorityName = (p: Priority | null | undefined) => PRIORITIES.find((x) => x.id === (p ?? null))!.name;
// Signal bars filled for each level; Urgent draws its own mark.
export const PRIORITY_BARS: Record<Priority, number> = { urgent: 3, high: 3, normal: 2, low: 1 };

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
export const MONTH_NAMES = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
const pad = (n: number) => String(n).padStart(2, '0');
export const dayOf = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
export const dateOf = (day: string) => {
  const [y, m, d] = day.split('-').map(Number);
  return new Date(y!, m! - 1, d!);
};
const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);

// The picker's shortcuts. End of this week is the coming Sunday (today on a Sunday).
export function quickDues(now = new Date()): { name: string; day: string }[] {
  return [
    { name: 'Today', day: dayOf(now) },
    { name: 'Tomorrow', day: dayOf(addDays(now, 1)) },
    { name: 'End of this week', day: dayOf(addDays(now, (7 - now.getDay()) % 7)) },
    { name: 'In one week', day: dayOf(addDays(now, 7)) },
  ];
}

// "Today", "Tomorrow", "Yesterday", "Nov 20", or "Nov 20, 2027" in another year.
export function dueLabel(day: string, now = new Date()): string {
  const today = dayOf(now);
  if (day === today) return 'Today';
  if (day === dayOf(addDays(now, 1))) return 'Tomorrow';
  if (day === dayOf(addDays(now, -1))) return 'Yesterday';
  const d = dateOf(day);
  return `${MONTHS[d.getMonth()]} ${d.getDate()}${d.getFullYear() === now.getFullYear() ? '' : `, ${d.getFullYear()}`}`;
}
export const overdue = (day: string, now = new Date()) => day < dayOf(now);
export const shortDay = (day: string) => {
  const d = dateOf(day);
  return `${['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'][d.getDay()]}, ${MONTHS[d.getMonth()]} ${d.getDate()}`;
};

// Six rows of seven days, weeks starting Monday, for the month holding `month` (YYYY-MM).
export function monthGrid(month: string): { day: string; inMonth: boolean }[] {
  const [y, m] = month.split('-').map(Number);
  const first = new Date(y!, m! - 1, 1);
  const start = addDays(first, -((first.getDay() + 6) % 7));
  return Array.from({ length: 42 }, (_, i) => {
    const d = addDays(start, i);
    return { day: dayOf(d), inMonth: d.getMonth() === m! - 1 };
  });
}
export const shiftMonth = (month: string, by: number) => {
  const [y, m] = month.split('-').map(Number);
  const d = new Date(y!, m! - 1 + by, 1);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}`;
};

// "1/2 closed" for a task's sub-tasks, or null with none.
export function subTaskProgress(children: { status: string }[]): string | null {
  if (!children.length) return null;
  return `${children.filter((c) => c.status === 'done').length}/${children.length}`;
}

// A pasted address as an https link, or null.
export function cleanUrl(text: string): string | null {
  const t = text.trim();
  if (!t || /\s/.test(t)) return null;
  const url = /^https?:\/\//i.test(t) ? t : /^[\w-]+(\.[\w-]+)+(\/|$)/.test(t) ? `https://${t}` : null;
  try {
    return url && new URL(url).hostname ? url : null;
  } catch {
    return null;
  }
}
