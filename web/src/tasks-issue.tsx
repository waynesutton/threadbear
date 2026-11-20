// The task editor's issue parts (backend API 87): priority and due date
// pickers, sub-tasks, links and activity with comments, and the icons rows
// and cards show. Drawn to match tasks.css; both themes come from its colors.
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { useMutation, useQuery } from 'convex/react';
import type { FunctionReturnType } from 'convex/server';
import type { Id } from '../../convex/_generated/dataModel';
import { api } from '../../convex/_generated/api';
import { go } from './route';
import { MONTH_NAMES, PRIORITIES, PRIORITY_BARS, cleanUrl, dateOf, dayOf, dueLabel, monthGrid, overdue, priorityName, quickDues, shiftMonth, shortDay, subTaskProgress, timeLabel, type Priority } from './tasks-fields';

type Task = FunctionReturnType<typeof api.tasks.list>['items'][number];
type Link = NonNullable<Task['links']>[number];
type Write = (work: () => Promise<unknown>, optimistic?: (rows: Task[]) => Task[]) => Promise<boolean>;

// Urgent: a red square with "!". High, Normal, Low: three bars with 3, 2 or 1 filled. None: dashes.
export function PriorityIcon({ priority, size = 14 }: { priority: Priority | null | undefined; size?: number }) {
  const common = { width: size, height: size, viewBox: '0 0 14 14', 'aria-hidden': true, className: `priority-icon priority-${priority ?? 'none'}` } as const;
  if (priority === 'urgent')
    return (
      <svg {...common}>
        <rect x="1" y="1" width="12" height="12" rx="3" fill="var(--red)" />
        <path d="M7 3.8v4" stroke="#fff" strokeWidth="1.7" strokeLinecap="round" />
        <circle cx="7" cy="10.2" r="0.95" fill="#fff" />
      </svg>
    );
  if (!priority)
    return (
      <svg {...common}>
        {[2, 6, 10].map((x) => <rect key={x} x={x - 0.75} y="6.25" width="2.5" height="1.5" rx="0.5" fill="currentColor" opacity="0.6" />)}
      </svg>
    );
  const filled = PRIORITY_BARS[priority];
  return (
    <svg {...common}>
      {[0, 1, 2].map((i) => (
        <rect key={i} x={1.5 + i * 4} y={9 - i * 3} width="3" height={3 + i * 3} rx="0.8" fill="currentColor" opacity={i < filled ? 1 : 0.25} />
      ))}
    </svg>
  );
}
const CalendarIcon = () => (
  <svg width="13" height="13" viewBox="0 0 14 14" aria-hidden="true">
    <rect x="1.75" y="2.75" width="10.5" height="9.5" rx="2" fill="none" stroke="currentColor" strokeWidth="1.2" />
    <path d="M1.75 5.75h10.5M4.75 1.5v2.5M9.25 1.5v2.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
  </svg>
);
const Check = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true" className="issue-check">
    <path d="M2.5 6.2 5 8.6l4.5-5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

// The chip rows and cards show: the due day, red once it's passed and the task isn't done.
export function DueChip({ day, time, done }: { day: string; time?: string | null; done: boolean }) {
  const late = !done && overdue(day, new Date(), time);
  const at = time ? ` ${timeLabel(time)}` : '';
  return (
    <span className={`task-due ${late ? 'is-late' : ''}`} title={`Due ${shortDay(day)}${at}${late ? ', overdue' : ''}`}>
      <CalendarIcon />
      {dueLabel(day)}{at}
    </span>
  );
}
export function SubTaskChip({ items }: { items: Task[] }) {
  const progress = subTaskProgress(items);
  if (!progress) return null;
  return (
    <span className="task-due" title={`${progress} sub-tasks closed`}>
      <svg width="12" height="12" viewBox="0 0 14 14" aria-hidden="true">
        <path d="M3 2v6.5a2 2 0 0 0 2 2h6M8.5 8l2.5 2.5L8.5 13" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
      {progress}
    </span>
  );
}

// A small menu under its pill, closed by Escape or a click outside.
function Popover({ label, button, children, open, setOpen }: { label: string; button: ReactNode; children: ReactNode; open: boolean; setOpen: (o: boolean) => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => !ref.current?.contains(e.target as Node) && setOpen(false);
    document.addEventListener('mousedown', close);
    requestAnimationFrame(() => (ref.current?.querySelector<HTMLElement>('.issue-pop [aria-checked="true"]') ?? ref.current?.querySelector<HTMLElement>('.issue-pop button'))?.focus());
    return () => document.removeEventListener('mousedown', close);
  }, [open, setOpen]);
  return (
    <div
      className="issue-anchor"
      ref={ref}
      onKeyDown={(e) => {
        if (e.key === 'Escape' && open) {
          e.preventDefault();
          e.stopPropagation();
          setOpen(false);
          ref.current?.querySelector<HTMLElement>('.task-pill')?.focus();
        }
      }}
    >
      {button}
      {open && (
        <div className="issue-pop" role="dialog" aria-label={label}>
          {children}
        </div>
      )}
    </div>
  );
}

export function PriorityPicker({ value, onChange, disabled }: { value: Priority | null | undefined; onChange: (p: Priority | null) => void; disabled?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <Popover
      label="Priority"
      open={open}
      setOpen={setOpen}
      button={
        <button type="button" className="task-pill is-button" title="Set the priority" aria-haspopup="menu" aria-expanded={open} disabled={disabled} onClick={() => setOpen(!open)}>
          <PriorityIcon priority={value} />
          <span>{value ? priorityName(value) : 'Priority'}</span>
        </button>
      }
    >
      <div role="menu" aria-label="Priority">
        {PRIORITIES.map((p) => (
          <button
            key={p.id ?? 'none'}
            type="button"
            role="menuitemradio"
            aria-checked={(value ?? null) === p.id}
            title={p.id ? `Set priority to ${p.name}` : 'Remove the priority'}
            onClick={() => {
              onChange(p.id);
              setOpen(false);
            }}
          >
            <PriorityIcon priority={p.id} />
            <span>{p.name}</span>
            {(value ?? null) === p.id && <Check />}
          </button>
        ))}
      </div>
    </Popover>
  );
}

export function DuePicker({ value, onChange, disabled }: { value: string | null | undefined; onChange: (d: string | null) => void; disabled?: boolean }) {
  const [open, setOpen] = useState(false);
  const [month, setMonth] = useState(() => (value ?? dayOf(new Date())).slice(0, 7));
  const today = dayOf(new Date());
  const pick = (d: string | null) => {
    onChange(d);
    setOpen(false);
  };
  const [y, m] = month.split('-').map(Number);
  return (
    <Popover
      label="Due date"
      open={open}
      setOpen={setOpen}
      button={
        <button type="button" className={`task-pill is-button ${value && overdue(value) ? 'is-late' : ''}`} title="Set the due date" aria-expanded={open} disabled={disabled} onClick={() => {
          // Opens on the chosen day's month, or this one.
          if (!open) setMonth((value ?? dayOf(new Date())).slice(0, 7));
          setOpen(!open);
        }}>
          <CalendarIcon />
          <span>{value ? dueLabel(value) : 'Due date'}</span>
        </button>
      }
    >
      <div className="issue-quick" role="menu" aria-label="Quick dates">
        {quickDues().map((q) => (
          <button key={q.name} type="button" role="menuitemradio" aria-checked={value === q.day} title={`Due ${shortDay(q.day)}`} onClick={() => pick(q.day)}>
            <span>{q.name}</span>
            <span className="issue-hint">{shortDay(q.day)}</span>
          </button>
        ))}
      </div>
      <div className="issue-cal">
        <div className="issue-cal-head">
          <button type="button" title="Previous month" aria-label="Previous month" onClick={() => setMonth(shiftMonth(month, -1))}>
            ‹
          </button>
          <span>
            {MONTH_NAMES[m! - 1]} {y}
          </span>
          <button type="button" title="Next month" aria-label="Next month" onClick={() => setMonth(shiftMonth(month, 1))}>
            ›
          </button>
        </div>
        <div className="issue-cal-grid" role="grid" aria-label={`${MONTH_NAMES[m! - 1]} ${y}`}>
          {['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'].map((d) => (
            <span key={d} className="issue-cal-dow" aria-hidden="true">
              {d}
            </span>
          ))}
          {monthGrid(month).map((d) => (
            <button
              key={d.day}
              type="button"
              role="gridcell"
              className={`${d.inMonth ? '' : 'is-out'} ${d.day === today ? 'is-today' : ''} ${d.day === value ? 'is-picked' : ''}`}
              aria-selected={d.day === value}
              title={shortDay(d.day)}
              onClick={() => pick(d.day)}
            >
              {dateOf(d.day).getDate()}
            </button>
          ))}
        </div>
      </div>
      {value && (
        <button type="button" className="issue-clear" title="Remove the due date" onClick={() => pick(null)}>
          Remove due date
        </button>
      )}
    </Popover>
  );
}

// Enter in these fields does their own thing, never submits the editor's form.
const onEnter = (e: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>, run: () => void) => {
  if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
    e.preventDefault();
    e.stopPropagation();
    run();
  }
};

export function IssueSections(props: {
  token: string;
  task: Task;
  tasks: Task[];
  disabled: boolean;
  write: Write;
  open: (t: Task) => void;
  statusIcon: (t: Task) => ReactNode;
  toggle: (t: Task) => void;
}) {
  const { token, task, tasks, disabled, write } = props;
  // One object for the component's life (per token): the React compiler takes a
  // fresh one each render as something handlers that capture it could change,
  // and then reads their Date.now() as running during render.
  const auth = useMemo(() => ({ token }), [token]);
  const create = useMutation(api.tasks.create);
  const update = useMutation(api.tasks.update);
  const upload = useMutation(api.tasks.uploadPhoto);
  const addImage = useMutation(api.tasks.addImage);
  const [subDraft, setSubDraft] = useState('');
  const [linkDraft, setLinkDraft] = useState<{ url: string; title: string } | null>(null);
  const parent = task.parentId ? tasks.find((t) => t.id === task.parentId) : undefined;
  const children = tasks.filter((t) => t.parentId === task.id).sort((a, b) => a.createdAt - b.createdAt);
  const links = task.links ?? [];

  function addSubTask() {
    const title = subDraft.trim();
    if (!title || disabled) return;
    const id = crypto.randomUUID();
    const now = Date.now();
    const row: Task = { id, title, notes: '', status: 'todo', columnId: 'todo', project: task.project, position: now, createdAt: now, updatedAt: now, deleted: false, parentId: task.id };
    void write(() => create({ auth, id, title, status: 'todo', project: task.project, parentId: task.id }), (rows) => [...rows, row]).then((ok) => ok && setSubDraft(''));
  }
  const setLinks = (next: Link[]) =>
    write(
      () => update({ auth, id: task.id, links: next.map(({ imageUrl: _url, ...l }) => l) }),
      (rows) => rows.map((t) => (t.id === task.id ? { ...t, links: next } : t)),
    );
  function addLink() {
    if (!linkDraft) return;
    const url = cleanUrl(linkDraft.url);
    if (!url) return;
    void setLinks([...links, { id: crypto.randomUUID(), kind: 'link', title: linkDraft.title.trim() || url, url }]).then((ok) => ok && setLinkDraft(null));
  }
  async function addPicture(file: File) {
    await write(async () => {
      if (file.size > 10 * 1024 * 1024 || !['image/png', 'image/jpeg', 'image/webp', 'image/gif'].includes(file.type)) throw new Error('Choose a PNG, JPEG, WebP or GIF under 10 MB.');
      const url = await upload({ auth });
      const response = await fetch(url, { method: 'POST', body: file, headers: { 'Content-Type': file.type } });
      if (!response.ok) throw new Error('Image upload failed. Try again.');
      const { storageId } = (await response.json()) as { storageId: Id<'_storage'> };
      await addImage({ auth, id: task.id, storageId, title: file.name.slice(0, 300) });
    });
  }
  const openLink = (l: Link) => {
    if (l.kind === 'thread' && l.thread) go(`/t/${encodeURIComponent(l.thread.id)}`);
    else if (l.url || l.imageUrl) window.open(l.url ?? l.imageUrl, '_blank', 'noopener');
  };
  const linkKind = { link: 'Link', print: 'Print', image: 'Image', thread: 'Thread' } as const;

  return (
    <>
      {parent && (
        <div className="issue-parent">
          <span>Sub-task of</span>
          <button type="button" className="issue-link-title" title={`Open ${parent.title}`} onClick={() => props.open(parent)}>
            {parent.title}
          </button>
          <button
            type="button"
            className="issue-remove"
            title="Make it a task of its own"
            aria-label="Remove from parent"
            disabled={disabled}
            onClick={() => void write(() => update({ auth, id: task.id, parentId: null }), (rows) => rows.map((t) => (t.id === task.id ? { ...t, parentId: null } : t)))}
          >
            ×
          </button>
        </div>
      )}
      {!task.parentId && (
        <section className="issue-section" aria-label="Sub-tasks">
          <h3>
            Sub-tasks
            {children.length > 0 && <span className="count">{subTaskProgress(children)} closed</span>}
          </h3>
          {children.map((c) => (
            <div key={c.id} className={`issue-sub ${c.status === 'done' ? 'is-done' : ''}`}>
              <button type="button" className="issue-sub-status" title={c.status === 'done' ? 'Mark not done' : 'Mark done'} aria-label={`Mark ${c.title} ${c.status === 'done' ? 'not done' : 'done'}`} disabled={disabled} onClick={() => props.toggle(c)}>
                {props.statusIcon(c)}
              </button>
              <button type="button" className="issue-link-title" title="Open sub-task" onClick={() => props.open(c)}>
                {c.title}
              </button>
              {c.priority && <PriorityIcon priority={c.priority} />}
              {c.dueDate && <DueChip day={c.dueDate} time={c.dueTime} done={c.status === 'done'} />}
            </div>
          ))}
          <input className="issue-add" aria-label="New sub-task" placeholder="Add a sub-task, then Enter" maxLength={500} value={subDraft} disabled={disabled} onChange={(e) => setSubDraft(e.target.value)} onKeyDown={(e) => onEnter(e, addSubTask)} />
        </section>
      )}
      <section className="issue-section" aria-label="Links">
        <h3>
          Links
          <span className="spacer" />
          <button type="button" className="issue-small" title="Add a web link" disabled={disabled || links.length >= 50} onClick={() => setLinkDraft({ url: '', title: '' })}>
            Add link
          </button>
          <label className={`issue-small tasks-file ${disabled || links.length >= 50 ? 'is-disabled' : ''}`} title="Add a PNG, JPEG, WebP or GIF under 10 MB">
            <input
              type="file"
              accept="image/png,image/jpeg,image/webp,image/gif"
              disabled={disabled || links.length >= 50}
              onChange={(e) => {
                const file = e.target.files?.[0];
                e.target.value = '';
                if (file) void addPicture(file);
              }}
            />
            Add image
          </label>
        </h3>
        {links.map((l) => (
          <div key={l.id} className="issue-linkrow">
            {l.kind === 'image' && l.imageUrl ? <img className="issue-thumb" src={l.imageUrl} alt="" /> : <span className={`issue-kind kind-${l.kind}`}>{linkKind[l.kind]}</span>}
            <button type="button" className="issue-link-title" title={l.url ?? (l.kind === 'thread' ? 'Open the thread' : l.title)} disabled={!l.url && !l.imageUrl && l.kind !== 'thread'} onClick={() => openLink(l)}>
              {l.title}
            </button>
            <button type="button" className="issue-remove" title="Remove this link" aria-label={`Remove ${l.title}`} disabled={disabled} onClick={() => void setLinks(links.filter((x) => x.id !== l.id))}>
              ×
            </button>
          </div>
        ))}
        {linkDraft && (
          <div className="issue-linkform">
            <input autoFocus aria-label="Link address" placeholder="https://" value={linkDraft.url} maxLength={2000} onChange={(e) => setLinkDraft({ ...linkDraft, url: e.target.value })} onKeyDown={(e) => onEnter(e, addLink)} />
            <input aria-label="Link title" placeholder="Title (optional)" value={linkDraft.title} maxLength={300} onChange={(e) => setLinkDraft({ ...linkDraft, title: e.target.value })} onKeyDown={(e) => onEnter(e, addLink)} />
            <button type="button" title="Add this link" disabled={disabled || !cleanUrl(linkDraft.url)} onClick={addLink}>
              Add
            </button>
            <button type="button" title="Don't add a link" onClick={() => setLinkDraft(null)}>
              Cancel
            </button>
          </div>
        )}
        {!links.length && !linkDraft && <p className="issue-empty">No links yet.</p>}
      </section>
      <Activity token={token} task={task} disabled={disabled} write={write} />
    </>
  );
}

function Activity({ token, task, disabled, write }: { token: string; task: Task; disabled: boolean; write: Write }) {
  const auth = { token };
  const activity = useQuery(api.tasks.activity, { auth, id: task.id });
  const comment = useMutation(api.tasks.comment);
  const removeComment = useMutation(api.tasks.removeComment);
  const [only, setOnly] = useState<'all' | 'comments'>('all');
  const [draft, setDraft] = useState('');
  const items = (activity?.items ?? []).filter((i) => only === 'all' || i.kind === 'comment');
  const send = () => {
    const body = draft.trim();
    if (!body || disabled) return;
    void write(() => comment({ auth, id: task.id, body })).then((ok) => ok && setDraft(''));
  };
  const when = (at: number) => {
    const d = new Date(at);
    const day = dayOf(d);
    return `${dueLabel(day)}${day === dayOf(new Date()) ? ` at ${d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}` : ''}`;
  };
  return (
    <section className="issue-section issue-activity" aria-label="Activity">
      <h3>
        Activity
        <span className="spacer" />
        <span className="tasks-segment" role="group" aria-label="Show">
          {(['all', 'comments'] as const).map((o) => (
            <button key={o} type="button" title={o === 'all' ? 'Show comments and changes' : 'Show only comments'} aria-pressed={only === o} onClick={() => setOnly(o)}>
              {o === 'all' ? 'All' : 'Comments'}
            </button>
          ))}
        </span>
      </h3>
      {activity === undefined ? (
        <p className="issue-empty">Loading…</p>
      ) : !items.length ? (
        <p className="issue-empty">{only === 'comments' ? 'No comments yet.' : 'Nothing yet.'}</p>
      ) : (
        <ol className="issue-feed">
          {items.map((i) =>
            i.kind === 'event' ? (
              <li key={i.id} className="issue-event">
                <span className={`issue-avatar is-${i.author.kind}`} aria-hidden="true">
                  {i.author.kind === 'agent' ? '◆' : i.author.name.slice(0, 1).toUpperCase()}
                </span>
                <span>
                  <strong>{i.author.name}</strong> {i.body}
                </span>
                <time className="issue-hint" dateTime={new Date(i.createdAt).toISOString()}>
                  {when(i.createdAt)}
                </time>
              </li>
            ) : (
              <li key={i.id} className="issue-comment">
                <header>
                  <span className={`issue-avatar is-${i.author.kind}`} aria-hidden="true">
                    {i.author.kind === 'agent' ? '◆' : i.author.name.slice(0, 1).toUpperCase()}
                  </span>
                  <strong>{i.author.name}</strong>
                  {i.author.kind === 'agent' && <span className="task-chip">Agent</span>}
                  <time className="issue-hint" dateTime={new Date(i.createdAt).toISOString()}>
                    {when(i.createdAt)}
                  </time>
                  <span className="spacer" />
                  <button type="button" className="issue-remove" title="Delete this comment" aria-label="Delete comment" disabled={disabled} onClick={() => void write(() => removeComment({ auth, id: task.id, commentId: i.id }))}>
                    ×
                  </button>
                </header>
                <p>{i.body}</p>
              </li>
            ),
          )}
        </ol>
      )}
      <div className="issue-commentbox">
        <textarea
          aria-label="Comment"
          placeholder="Leave a comment…"
          rows={2}
          maxLength={10000}
          value={draft}
          disabled={disabled}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            // Command-Enter comments here rather than saving the task.
            if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              e.stopPropagation();
              send();
            }
          }}
        />
        <button type="button" className="is-primary" title="Comment (⌘Enter)" disabled={disabled || !draft.trim()} onClick={send}>
          Comment
        </button>
      </div>
    </section>
  );
}
