import { useEffect, useRef, useState, type CSSProperties, type FormEvent, type ReactNode } from 'react';
import { useConvex, useConvexConnectionState, useMutation, useQuery } from 'convex/react';
import type { FunctionReturnType } from 'convex/server';
import type { Id } from '../../convex/_generated/dataModel';
import { api } from '../../convex/_generated/api';
import './tasks.css';
import { columnDefinitions, orderedColumns, taskColumn } from './tasks-columns';
import { go } from './route';
import { applyTheme, readTheme } from './theme';
import { taskDraftChanges } from './tasks-draft';
import { LIST_STATUSES, fullOrder, groupOrder, moveTo, positionAt, splitDraft } from './tasks-order';

type Task = FunctionReturnType<typeof api.tasks.list>['items'][number];
type Status = Task['status'];
type Prefs = FunctionReturnType<typeof api.tasks.preferences>;
type Project = { runnerId: string; id: string; title: string };
const projectKey = (p: Task['project']) => (p ? `${p.runnerId}/${p.id}` : 'none');
const NONE = 'none';

// Linear's status glyphs, drawn small so a row reads at a glance.
export function StatusIcon({ status }: { status: Status }) {
  const common = { width: 14, height: 14, viewBox: '0 0 14 14', 'aria-hidden': true, className: `status-icon status-${status}` };
  if (status === 'backlog')
    return (
      <svg {...common}>
        <circle cx="7" cy="7" r="5.5" fill="none" stroke="currentColor" strokeWidth="1.5" strokeDasharray="2 2" />
      </svg>
    );
  if (status === 'todo')
    return (
      <svg {...common}>
        <circle cx="7" cy="7" r="5.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
      </svg>
    );
  if (status === 'doing')
    return (
      <svg {...common}>
        <circle cx="7" cy="7" r="5.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
        <path d="M7 3.5a3.5 3.5 0 0 1 0 7z" fill="currentColor" />
      </svg>
    );
  return (
    <svg {...common}>
      <circle cx="7" cy="7" r="6.25" fill="currentColor" />
      <path d="M4.4 7.2 6.2 9l3.4-3.6" fill="none" stroke="var(--raised)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

const Dots = () => (
  <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
    <circle cx="3" cy="7" r="1.2" fill="currentColor" />
    <circle cx="7" cy="7" r="1.2" fill="currentColor" />
    <circle cx="11" cy="7" r="1.2" fill="currentColor" />
  </svg>
);
const NoteIcon = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true" className="task-note-icon">
    <path d="M2.5 2.5h7M2.5 5.5h7M2.5 8.5h4.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
  </svg>
);

type Drag = { kind: 'task' | 'status' | 'project'; id: string; label: string };
type Menu = { task: Task; x: number; y: number; page: 'main' | 'project' };

function TasksContent({ token, projectFilter = '' }: { token: string; projectFilter?: string }) {
  const auth = { token };
  const convex = useConvex();
  const connection = useConvexConnectionState();
  const revision = useQuery(api.tasks.revision, { auth });
  const prefs = useQuery(api.tasks.preferences, { auth });
  const photo = useQuery(api.tasks.photo, { auth });
  const projects: Project[] = useQuery(api.phone.projects, { token }) ?? [];
  const projectIds = projects
    .map((p) => `${p.runnerId}/${p.id}`)
    .sort()
    .join('|');
  const create = useMutation(api.tasks.create);
  const update = useMutation(api.tasks.update);
  const remove = useMutation(api.tasks.remove);
  const restore = useMutation(api.tasks.restore);
  const startThread = useMutation(api.tasks.startThread);
  const setPrefs = useMutation(api.tasks.setPreferences);
  const upload = useMutation(api.tasks.uploadPhoto);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(0);
  const [currentView, setView] = useState<'list' | 'board' | null>(null);
  const [filter, setFilter] = useState(projectFilter || '');
  const editingBaseline = useRef<Task | null>(null);
  const [editing, setEditing] = useState<(Partial<Task> & { focusNotes?: boolean }) | null>(null);
  const [panel, setPanel] = useState<'display' | 'customize' | null>(null);
  const [toast, setToast] = useState<{ text: string; undo: () => Promise<unknown> } | null>(null);
  const [menu, setMenu] = useState<Menu | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Task | null>(null);
  const deleteRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!pendingDelete) return;
    const previous = document.activeElement as HTMLElement | null;
    deleteRef.current?.querySelector<HTMLButtonElement>('button')?.focus();
    return () => previous?.focus();
  }, [pendingDelete]);
  const [showDone, setShowDone] = useState(() => localStorage.getItem('tasks.showDone') === '1');
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());
  const [draft, setDraft] = useState('');
  const [columnDraft, setColumnDraft] = useState<{ group: string; status: string; text: string } | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [over, setOver] = useState<string | null>(null);
  const [ghost, setGhost] = useState<{ x: number; y: number; label: string } | null>(null);
  const editorRef = useRef<HTMLFormElement>(null);
  const composerRef = useRef<HTMLTextAreaElement>(null);
  const view = currentView ?? prefs?.defaultView ?? 'list';
  const online = connection.isWebSocketConnected;
  const disabled = !online || saving > 0;
  const definitions = columnDefinitions(prefs?.columns);
  const names = Object.fromEntries(definitions.map(c => [c.id, c.name]));
  const canonical = (id: string): Status => definitions.find(c => c.id === id)?.status ?? 'todo';
  const membership = (t: Task) => taskColumn(t, definitions);
  const columns = orderedColumns(definitions, prefs?.boardColumnOrder ?? fullOrder(prefs?.columnOrder));
  const listOrder = orderedColumns(definitions, prefs?.sectionOrder ?? fullOrder(prefs?.listOrder, LIST_STATUSES));
  const [columnEditor, setColumnEditor] = useState<{ id: string; name: string; fresh: boolean } | null>(null);
  const columnReturnFocus = useRef<HTMLElement | null>(null);
  function closeColumnEditor() { setColumnEditor(null); requestAnimationFrame(() => columnReturnFocus.current?.focus()); }

  const order = view === 'list' ? listOrder : columns;
  const manual = (prefs?.sort ?? 'manual') === 'manual';
  const byProject = prefs?.grouping === 'project';

  useEffect(() => {
    if (!prefs?.theme) return;
    applyTheme(prefs.theme);
    return () => applyTheme(readTheme());
  }, [prefs?.theme]);
  useEffect(() => {
    if (!editing) return;
    const previous = document.activeElement as HTMLElement | null;
    editorRef.current?.querySelector<HTMLElement>(editing.focusNotes ? 'textarea' : 'input')?.focus();
    return () => previous?.focus();
  }, [editing?.id]);
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), 6000);
    return () => clearTimeout(timer);
  }, [toast]);
  useEffect(() => {
    if (!menu && !panel) return;
    const close = (e: MouseEvent | KeyboardEvent) => {
      if (e instanceof KeyboardEvent ? e.key === 'Escape' : !(e.target as HTMLElement).closest('.task-menu, .tasks-popover, [data-popover-toggle]')) {
        setMenu(null);
        setPanel(null);
      }
    };
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', close);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', close);
    };
  }, [menu, panel]);
  // Reconcile complete pages on an external realtime invalidation, retaining the
  // last successful snapshot while disconnected. Never persist tokens or notes.
  useEffect(() => {
    if (revision === undefined) return;
    let cancelled = false;
    void (async () => {
      const all: Task[] = [];
      let after: string | undefined;
      do {
        const page = await convex.query(api.tasks.list, { auth: { token }, after, limit: 250 });
        all.push(...page.items);
        after = page.next ?? undefined;
      } while (after && !cancelled);
      if (!cancelled) {
        setTasks(all);
        setLoaded(true);
      }
    })().catch((e) => {
      if (!cancelled) setError(String(e));
    });
    return () => {
      cancelled = true;
    };
  }, [convex, token, revision, online, projectIds]);

  // Shows the change at once and puts it back, with the error, if the write fails.
  async function write(work: () => Promise<unknown>, optimistic?: (rows: Task[]) => Task[]): Promise<boolean> {
    if (!online) return false;
    const before = tasks;
    if (optimistic) setTasks(optimistic);
    setSaving((n) => n + 1);
    setError('');
    try {
      await work();
      return true;
    } catch (e) {
      if (optimistic) setTasks(before);
      setError(e instanceof Error ? e.message.replace(/^.*Uncaught (ConvexError: )?/, '') : String(e));
      return false;
    } finally {
      setSaving((n) => n - 1);
    }
  }
  const patchTask = (id: string, patch: Partial<Task>) => {
    const fields = Object.fromEntries(Object.entries(patch).filter(([k]) => ['title', 'notes', 'status', 'columnId', 'project', 'position'].includes(k)));
    return write(
      () => update({ auth, id, ...fields }),
      (rows) => rows.map((t) => (t.id === id ? { ...t, ...patch } : t)),
    );
  };
  const preference = (patch: Partial<Prefs>) => void write(() => setPrefs({ auth, ...patch }));
  const findProject = (key: string) => projects.find((p) => `${p.runnerId}/${p.id}` === key);
  const projectName = (key: string) => (key === NONE ? 'No project' : (findProject(key)?.title ?? 'Unknown project'));
  const projectRef = (key: string) => {
    const p = findProject(key);
    return p ? { runnerId: p.runnerId, id: p.id } : null;
  };

  const visible = tasks.filter((t) => !filter || projectKey(t.project) === filter);
  visible.sort((a, b) =>
    prefs?.sort === 'title'
      ? a.title.localeCompare(b.title)
      : prefs?.sort === 'newest'
        ? b.createdAt - a.createdAt
        : prefs?.sort === 'oldest'
          ? a.createdAt - b.createdAt
          : a.position - b.position || a.id.localeCompare(b.id),
  );
  const doneCount = visible.filter((t) => t.status === 'done').length;
  const open = visible.filter((t) => t.status !== 'done');
  const groups = byProject ? groupOrder(prefs?.projectOrder, visible.map((t) => projectKey(t.project)), filter) : ['all'];
  const inGroup = (t: Task, group: string) => group === 'all' || projectKey(t.project) === group;

  function addTask(text: string, columnId = 'todo', group = 'all') {
    const status = canonical(columnId);
    const { title, notes } = splitDraft(text);
    if (!title) return Promise.resolve(false);
    const key = group !== 'all' ? group : filter;
    const project = key ? projectRef(key) : null;
    const id = crypto.randomUUID();
    const peers = visible.filter((t) => t.status === status);
    const position = peers.length ? Math.min(...peers.map((t) => t.position)) - 1000 : Date.now();
    const now = Date.now();
    const row: Task = { id, title, notes, status, columnId, project, position, createdAt: now, updatedAt: now, deleted: false };
    return write(
      // New tasks go on top, where the composer is.
      () => create({ auth, id, title, notes, status, columnId, project, position }),
      (rows) => [...rows, row],
    );
  }
  async function submitComposer(e?: FormEvent) {
    e?.preventDefault();
    const text = draft;
    if (!text.trim() || disabled) return;
    if (await addTask(text)) setDraft('');
    composerRef.current?.focus();
  }
  function complete(t: Task) {
    const was = t.status;
    const next: Status = was === 'done' ? 'todo' : 'done';
    void patchTask(t.id, { status: next }).then((ok) => {
      if (ok && next === 'done') setToast({ text: `Done: ${t.title}`, undo: () => patchTask(t.id, { status: was, columnId: t.columnId ?? was }) });
    });
  }
  function deleteTask(t: Task) { setPendingDelete(t); }
  function confirmDelete(t: Task) {
    void write(
      () => remove({ auth, id: t.id }),
      (rows) => rows.filter((r) => r.id !== t.id),
    ).then((ok) => {
      if (ok) {
        setPendingDelete(null);
        setEditing(null);
        setToast({ text: `Deleted: ${t.title}`, undo: () => write(() => restore({ auth, id: t.id }), (rows) => [...rows, t]) });
      }
    });
  }
  async function sendTask(t: Task) {
    if (t.thread) { go(`/t/${encodeURIComponent(t.thread.id)}`); return; }
    if (t.threadCommandId) { if (t.threadError) setError(t.threadError); return; }
    setMenu(null);
    await write(() => startThread({ auth, id: t.id }));
  }
  function threadLabel(t: Partial<Task>) { return t.thread ? 'View thread' : t.threadError ? 'Thread failed' : t.threadCommandId ? 'Starting thread…' : 'Send to thread'; }
  async function saveEditor(e: FormEvent) {
    e.preventDefault();
    if (!editing?.id || !editing.title?.trim()) return;
    const { id, title = '', notes = '', status = 'todo', columnId, project = null } = editing;
    const patch = { id, title, notes, status, columnId, project: project ? { id: project.id, runnerId: project.runnerId } : null };
    const base = editingBaseline.current;
    const ok = base
      ? await write(
          () => update({ auth, id, ...taskDraftChanges(base, patch) }),
          (rows) => rows.map((t) => (t.id === id ? { ...t, ...patch } : t)),
        )
      : await write(() => create({ auth, ...patch }));
    if (ok) setEditing(null);
  }
  const openEditor = (t: Task, focusNotes = false) => {
    editingBaseline.current = t;
    setEditing({ ...t, focusNotes });
    setMenu(null);
  };

  // One drop handler for everything that moves: tasks, status sections and project groups.
  function dropOn(target: { status?: string; group?: string; before?: Task }) {
    const d = drag;
    setDrag(null);
    setOver(null);
    if (!d || disabled) return;
    if (d.kind === 'status' && target.status && target.status !== d.id) {
      const next = moveTo(order, d.id, target.status);
      preference(view === 'list' ? { sectionOrder: next } : { boardColumnOrder: next });
    } else if (d.kind === 'project' && target.group && target.group !== d.id) {
      preference({ projectOrder: moveTo(groups, d.id, target.group) });
    } else if (d.kind === 'task') {
      const task = tasks.find((t) => t.id === d.id);
      if (!task || target.before?.id === task.id) return;
      const columnId = target.status ?? membership(task);
      const status = canonical(columnId);
      const group = target.group ?? 'all';
      const peers = visible.filter((t) => membership(t) === columnId && t.id !== task.id && inGroup(t, group));
      const patch: Partial<Task> = { status, columnId, position: manual ? positionAt(peers, target.before?.id) : task.position };
      if (group !== 'all') patch.project = projectRef(group);
      if (status === task.status && group === 'all' && !manual) return;
      void patchTask(task.id, patch);
    }
  }
  const dragProps = (d: Drag, enabled = true) =>
    enabled && !disabled
      ? {
          draggable: true,
          onDragStart: (e: React.DragEvent) => {
            e.stopPropagation();
            e.dataTransfer.effectAllowed = 'move';
            e.dataTransfer.setData('text/plain', d.label);
            setDrag(d);
          },
          onDragEnd: () => {
            setDrag(null);
            setOver(null);
          },
        }
      : {};
  const dropProps = (key: string, accepts: Drag['kind'][], target: Parameters<typeof dropOn>[0]) => ({
    'data-drop': key,
    'data-drop-kinds': accepts.join(' '),
    'data-drop-status': target.status,
    'data-drop-group': target.group,
    'data-drop-before': target.before?.id,
    onDragOver: (e: React.DragEvent) => {
      if (!drag || !accepts.includes(drag.kind)) return;
      e.preventDefault();
      e.stopPropagation();
      if (over !== key) setOver(key);
    },
    onDrop: (e: React.DragEvent) => {
      if (!drag || !accepts.includes(drag.kind)) return;
      e.preventDefault();
      e.stopPropagation();
      dropOn(target);
    },
  });
  // Touch has no HTML drag and drop, so a held grip drags a ghost and drops on what's under it.
  // Cards wait for a short hold, so a swipe still scrolls and a tap still opens.
  const hold = useRef<{ timer: number; x: number; y: number } | null>(null);
  const touchGrip = (d: Drag, holdMs = 0) => ({
    onPointerDown: (e: React.PointerEvent<HTMLElement>) => {
      if (e.pointerType === 'mouse' || disabled) return;
      const el = e.currentTarget;
      const { pointerId, clientX, clientY } = e;
      const start = () => {
        hold.current = null;
        try {
          el.setPointerCapture(pointerId);
        } catch {
          return;
        }
        setDrag(d);
        setGhost({ x: clientX, y: clientY, label: d.label });
        navigator.vibrate?.(10);
      };
      if (!holdMs) return start();
      hold.current = { timer: window.setTimeout(start, holdMs), x: clientX, y: clientY };
    },
    onPointerMove: (e: React.PointerEvent<HTMLElement>) => {
      if (hold.current && Math.hypot(e.clientX - hold.current.x, e.clientY - hold.current.y) > 8) {
        clearTimeout(hold.current.timer);
        hold.current = null;
      }
      if (ghost) setGhost({ x: e.clientX, y: e.clientY, label: d.label });
    },
    onPointerUp: (e: React.PointerEvent<HTMLElement>) => {
      if (hold.current) {
        clearTimeout(hold.current.timer);
        hold.current = null;
      }
      if (!ghost) return;
      e.preventDefault();
      setGhost(null);
      const el = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-drop]');
      if (el && el.dataset.dropKinds?.split(' ').includes(d.kind))
        dropOn({ status: el.dataset.dropStatus as Status | undefined, group: el.dataset.dropGroup, before: tasks.find((t) => t.id === el.dataset.dropBefore) });
      else setDrag(null);
    },
    onPointerCancel: () => {
      if (hold.current) clearTimeout(hold.current.timer);
      hold.current = null;
      setGhost(null);
      setDrag(null);
    },
  });
  function moveByKey(t: Task, direction: number, group: string) {
    const items = visible.filter((r) => membership(r) === membership(t) && inGroup(r, group));
    const index = items.findIndex((r) => r.id === t.id);
    const target = items[index + direction];
    if (!target) return;
    const rest = items.filter((r) => r.id !== t.id);
    const before = direction < 0 ? target : rest[rest.indexOf(target) + 1];
    void patchTask(t.id, { position: positionAt(rest, before?.id) });
  }
  const openMenu = (t: Task, e: React.MouseEvent<HTMLElement>) => {
    if (menu?.task.id === t.id) return setMenu(null);
    const r = e.currentTarget.getBoundingClientRect();
    const width = 220;
    setMenu({ task: t, x: Math.max(8, Math.min(r.right - width, innerWidth - width - 8)), y: r.bottom + 4, page: 'main' });
  };

  function row(t: Task, group: string) {
    const key = `task:${t.id}`;
    return (
      <div
        key={t.id}
        className={`task-row ${t.status === 'done' ? 'is-done' : ''} ${drag?.id === t.id ? 'is-dragging' : ''} ${over === key ? 'is-over' : ''}`}
        {...dragProps({ kind: 'task', id: t.id, label: t.title }, manual)}
        {...dropProps(key, ['task'], { status: membership(t), group, before: t })}
      >
        <button
          className="task-grip"
          title="Drag to move. Arrow keys move it up or down."
          aria-label={`Move ${t.title}`}
          disabled={disabled || !manual}
          {...touchGrip({ kind: 'task', id: t.id, label: t.title })}
          onKeyDown={(e) => {
            if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
              e.preventDefault();
              moveByKey(t, e.key === 'ArrowUp' ? -1 : 1, group);
            }
          }}
        >
          <svg width="8" height="12" viewBox="0 0 8 12" aria-hidden="true">
            {[2, 6, 10].flatMap((y) => [<circle key={`a${y}`} cx="2" cy={y} r="1" fill="currentColor" />, <circle key={`b${y}`} cx="6" cy={y} r="1" fill="currentColor" />])}
          </svg>
        </button>
        <label className="task-check" title={t.status === 'done' ? 'Mark not done' : 'Mark done'}>
          <input type="checkbox" checked={t.status === 'done'} disabled={disabled} aria-label={`Mark ${t.title} ${t.status === 'done' ? 'not done' : 'done'}`} onChange={() => complete(t)} />
          <span aria-hidden="true" />
        </label>
        <button className="task-title" title="Open task" onClick={() => openEditor(t)}>
          <span>{t.title}</span>
          {t.notes && <NoteIcon />}
        </button>
        {t.project && !byProject && !filter && <span className="task-chip">{projectName(projectKey(t.project))}</span>}
        <button className="task-more" title="More: project, note, status, delete" aria-label={`More for ${t.title}`} data-popover-toggle aria-haspopup="menu" aria-expanded={menu?.task.id === t.id} onClick={(e) => openMenu(t, e)}>
          <Dots />
        </button>
      </div>
    );
  }
  function card(t: Task, group: string) {
    const key = `task:${t.id}`;
    return (
      <article
        key={t.id}
        className={`task-card ${t.status === 'done' ? 'is-done' : ''} ${drag?.id === t.id ? 'is-dragging' : ''} ${over === key ? 'is-over' : ''}`}
        {...dragProps({ kind: 'task', id: t.id, label: t.title })}
        {...dropProps(key, ['task'], { status: membership(t), group, before: t })}
        {...touchGrip({ kind: 'task', id: t.id, label: t.title }, 350)}
        onContextMenu={(e) => ghost && e.preventDefault()}
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === 'Enter') openEditor(t);
          if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
            e.preventDefault();
            moveByKey(t, e.key === 'ArrowUp' ? -1 : 1, group);
          }
        }}
      >
        <button className="task-title" title="Open task" onClick={() => openEditor(t)}>
          {t.title}
        </button>
        <button className="task-more" title="More: project, note, status, delete" aria-label={`More for ${t.title}`} data-popover-toggle aria-haspopup="menu" aria-expanded={menu?.task.id === t.id} onClick={(e) => openMenu(t, e)}>
          <Dots />
        </button>
        {(t.notes || (t.project && !byProject && !filter)) && (
          <div className="task-meta">
            {t.notes && <NoteIcon />}
            {t.project && !byProject && !filter && <span className="task-chip">{projectName(projectKey(t.project))}</span>}
          </div>
        )}
      </article>
    );
  }
  function groupHeader(group: string, count: number, children?: ReactNode) {
    const key = `project:${group}`;
    const folded = collapsed.has(key);
    return (
      <header className={`tasks-group-head ${over === key ? 'is-over' : ''}`} {...dragProps({ kind: 'project', id: group, label: projectName(group) })} {...dropProps(key, ['project'], { group })}>
        <button className="task-grip" title="Drag to reorder projects" aria-label={`Move ${projectName(group)}`} {...touchGrip({ kind: 'project', id: group, label: projectName(group) })}>
          <svg width="8" height="12" viewBox="0 0 8 12" aria-hidden="true">
            {[2, 6, 10].flatMap((y) => [<circle key={`a${y}`} cx="2" cy={y} r="1" fill="currentColor" />, <circle key={`b${y}`} cx="6" cy={y} r="1" fill="currentColor" />])}
          </svg>
        </button>
        <button className="tasks-fold" title={folded ? 'Show this project' : 'Hide this project'} aria-expanded={!folded} onClick={() => toggleFold(key)}>
          <span className={`chev ${folded ? '' : 'open'}`} aria-hidden="true" />
          <span className="tasks-project-dot" aria-hidden="true" />
          {projectName(group)}
          <span className="count">{count}</span>
        </button>
        {children}
      </header>
    );
  }
  const toggleFold = (key: string) =>
    setCollapsed((c) => {
      const next = new Set(c);
      if (!next.delete(key)) next.add(key);
      return next;
    });

  function listSection(status: string, group: string) {
    const items = visible.filter((t) => membership(t) === status && inGroup(t, group));
    if (!items.length && !(drag?.kind === 'task' && status !== 'done')) return null;
    const key = `status:${group}:${status}`;
    const folded = collapsed.has(key);
    return (
      <section key={key} className={`tasks-section ${over === key ? 'is-over' : ''}`} {...dropProps(key, ['task', 'status'], { status, group })}>
        <h2 title="Drag to reorder sections" {...dragProps({ kind: 'status', id: status, label: names[status] })} {...touchGrip({ kind: 'status', id: status, label: names[status] }, 350)}>
          <button className="tasks-fold" title={folded ? `Show ${names[status]}` : `Hide ${names[status]}`} aria-expanded={!folded} onClick={() => toggleFold(key)}>
            <span className={`chev ${folded ? '' : 'open'}`} aria-hidden="true" />
            <StatusIcon status={canonical(status)} />
            {names[status]}
            <span className="count">{items.length}</span>
          </button>
        </h2>
        {!folded && items.map((t) => row(t, group))}
      </section>
    );
  }
  function listView() {
    if (!visible.length)
      return (
        <div className="tasks-empty">
          <StatusIcon status="todo" />
          <p>Nothing here yet. Type a task above and press Enter.</p>
        </div>
      );
    return groups.map((group) => {
      const statuses = order.filter((s) => canonical(s) !== 'done' || showDone);
      const count = (showDone ? visible : open).filter((t) => inGroup(t, group)).length;
      const body = statuses.map((s) => listSection(s, group));
      if (group === 'all') return <div key="all">{body}</div>;
      return (
        <div key={group} className="tasks-group">
          {groupHeader(group, count)}
          {!collapsed.has(`project:${group}`) && body}
        </div>
      );
    });
  }
  function boardColumns(group: string) {
    return (
      <div className="tasks-columns">
        {order.map((status) => {
          const items = visible.filter((t) => membership(t) === status && inGroup(t, group));
          const key = `column:${group}:${status}`;
          const adding = columnDraft?.group === group && columnDraft.status === status;
          return (
            <section key={status} onClick={e => { if (e.target === e.currentTarget && !disabled) setColumnDraft({ group, status, text: '' }); }} className={`task-column ${over === key ? 'is-over' : ''}`} {...dropProps(key, ['task', 'status'], { status, group })}>
              <h2 title="Drag to reorder columns. Arrow keys move it." tabIndex={0} {...dragProps({ kind: 'status', id: status, label: names[status] })} {...touchGrip({ kind: 'status', id: status, label: names[status] })}
                onKeyDown={(e) => {
                  if (!['ArrowLeft', 'ArrowRight'].includes(e.key)) return;
                  e.preventDefault();
                  const next = [...order];
                  const index = next.indexOf(status);
                  const target = index + (e.key === 'ArrowLeft' ? -1 : 1);
                  if (target < 0 || target >= next.length) return;
                  [next[index], next[target]] = [next[target], next[index]];
                  preference({ boardColumnOrder: next });
                }}
              >
                <StatusIcon status={canonical(status)} />
                <button className="task-column-name" title="Rename column" disabled={disabled} onClick={e => { columnReturnFocus.current = e.currentTarget; setColumnEditor({ id: status, name: names[status], fresh: false }); }}>{names[status]}</button>
                <span className="count">{items.length}</span>
                <button className="task-add-icon" title={`Add a task to ${names[status]}`} aria-label={`Add a task to ${names[status]}`} disabled={disabled} onClick={() => setColumnDraft({ group, status, text: '' })}>
                  +
                </button>
              </h2>
              {adding && (
                <form
                  className="task-inline-add"
                  onSubmit={(e) => {
                    e.preventDefault();
                    const text = columnDraft.text;
                    if (disabled || !text.trim()) return;
                    void addTask(text, status, group).then((ok) => {
                      if (ok) setColumnDraft((current) => current?.group === group && current.status === status ? { ...current, text: '' } : current);
                    });
                  }}
                >
                  <input
                    autoFocus
                    placeholder="Task title, then Enter"
                    value={columnDraft.text}
                    disabled={disabled}
                    maxLength={500}
                    onChange={(e) => setColumnDraft({ ...columnDraft, text: e.target.value })}
                    onKeyDown={(e) => e.key === 'Escape' && setColumnDraft(null)}
                    onBlur={() => !columnDraft.text.trim() && setColumnDraft(null)}
                  />
                </form>
              )}
              {items.map((t) => card(t, group))}
              {!adding && <button className="task-column-empty tasks-wide" title={`Add a task to ${names[status]}`} disabled={disabled} onClick={() => setColumnDraft({ group, status, text: '' })}>Add a task</button>}
            </section>
          );
        })}
        <button className="tasks-add-column" title="Add a column" disabled={disabled || definitions.length >= 32} onClick={e => { columnReturnFocus.current = e.currentTarget; setColumnEditor({ id: crypto.randomUUID(), name: '', fresh: true }); }}>Add column</button>
      </div>
    );
  }
  function boardView() {
    if (!byProject) return boardColumns('all');
    return groups.map((group) => (
      <div key={group} className="tasks-group tasks-lane">
        {groupHeader(group, visible.filter((t) => inGroup(t, group)).length)}
        {!collapsed.has(`project:${group}`) && boardColumns(group)}
      </div>
    ));
  }

  async function choosePhoto(file: File) {
    await write(async () => {
      if (file.size > 10 * 1024 * 1024) throw new Error('Choose a photo under 10 MB.');
      const url = await upload({ auth });
      const response = await fetch(url, { method: 'POST', body: file, headers: { 'Content-Type': file.type } });
      if (!response.ok) throw new Error('Photo upload failed. Try again.');
      const { storageId } = (await response.json()) as { storageId: Id<'_storage'> };
      await setPrefs({ auth, backgroundPhoto: storageId });
    });
  }
  const background = {
    '--tasks-color': 'var(--bg)',
    '--tasks-photo': photo ? `url(${JSON.stringify(photo)})` : 'none',
    '--tasks-fade': prefs?.fade ?? 0.85,
    '--tasks-position': prefs?.photoPosition ?? 'center',
  } as CSSProperties;

  return (
    <section className={`tasks-page tasks-${view} ${photo ? 'has-photo' : ''}`} style={background} aria-label="Tasks" onKeyDownCapture={(e) => {
      if (!['ArrowUp', 'ArrowDown'].includes(e.key) || e.altKey || e.metaKey || e.ctrlKey) return;
      const target = e.target as HTMLElement;
      if (target.closest('input, textarea, select, [contenteditable="true"], .task-grip, [role="dialog"], [role="alertdialog"]')) return;
      const titles = [...e.currentTarget.querySelectorAll<HTMLButtonElement>('.task-row .task-title, .task-card .task-title')];
      const current = target.closest('.task-row, .task-card')?.querySelector('.task-title');
      const index = titles.findIndex(t => t === current);
      const next = index < 0 ? (e.key === 'ArrowDown' ? 0 : titles.length - 1) : Math.max(0, Math.min(titles.length - 1, index + (e.key === 'ArrowDown' ? 1 : -1)));
      if (titles[next]) { e.preventDefault(); e.stopPropagation(); titles[next].focus(); titles[next].scrollIntoView({ block: 'nearest', inline: 'nearest' }); }
    }}>
      <div className="tasks-inner">
        <header className="tasks-header">
          <h1>{filter ? projectName(filter) : 'Tasks'}</h1>
          <span className="tasks-count">{open.length} open</span>
          <div className="tasks-actions">
            <div className="tasks-segment" role="group" aria-label="Task view">
              {(['list', 'board'] as const).map((v) => (
                <button key={v} title={`Show ${v === 'list' ? 'the list' : 'the board'}`} aria-pressed={view === v} onClick={() => setView(v)}>
                  {v === 'list' ? 'List' : 'Board'}
                </button>
              ))}
            </div>
            <div className="tasks-pop-anchor">
              <button data-popover-toggle title="Grouping, order and the default view" aria-expanded={panel === 'display'} onClick={() => setPanel(panel === 'display' ? null : 'display')}>
                Display
              </button>
              {panel === 'display' && (
                <div className="tasks-popover" role="dialog" aria-label="Display options">
                  <label className="tasks-field">
                    <span>Project</span>
                    <select title="Show one project" value={filter} onChange={(e) => setFilter(e.target.value)}>
                      <option value="">All projects</option>
                      <option value={NONE}>No project</option>
                      {projects.map((p) => (
                        <option key={`${p.runnerId}/${p.id}`} value={`${p.runnerId}/${p.id}`}>
                          {p.title}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="tasks-field">
                    <span>Grouping</span>
                    <select title="Group tasks" value={prefs?.grouping ?? 'status'} disabled={disabled} onChange={(e) => preference({ grouping: e.target.value as Prefs['grouping'] })}>
                      <option value="status">Status</option>
                      <option value="project">Project</option>
                    </select>
                  </label>
                  <label className="tasks-field">
                    <span>Ordering</span>
                    <select title="Order tasks" value={prefs?.sort ?? 'manual'} disabled={disabled} onChange={(e) => preference({ sort: e.target.value as Prefs['sort'] })}>
                      <option value="manual">Manual</option>
                      <option value="newest">Newest</option>
                      <option value="oldest">Oldest</option>
                      <option value="title">Title</option>
                    </select>
                  </label>
                  {view === 'list' && (
                    <label className="tasks-switch">
                      <span>Show done tasks</span>
                      <input
                        type="checkbox"
                        role="switch"
                        title="Show done tasks in the list"
                        checked={showDone}
                        onChange={(e) => {
                          setShowDone(e.target.checked);
                          localStorage.setItem('tasks.showDone', e.target.checked ? '1' : '0');
                        }}
                      />
                    </label>
                  )}
                  <button className="tasks-wide" title="Open Tasks in this view next time" disabled={disabled || view === prefs?.defaultView} onClick={() => preference({ defaultView: view })}>
                    {view === prefs?.defaultView ? `${view === 'list' ? 'List' : 'Board'} is the default` : `Make ${view === 'list' ? 'List' : 'Board'} the default`}
                  </button>
                </div>
              )}
            </div>
            <div className="tasks-pop-anchor">
              <button data-popover-toggle title="Background color and photo" aria-expanded={panel === 'customize'} onClick={() => setPanel(panel === 'customize' ? null : 'customize')}>
                Background
              </button>
              {panel === 'customize' && (
                <div className="tasks-popover" role="dialog" aria-label="Background">
                  <span className="tasks-label">Appearance</span>
                  <div className="tasks-segment" role="group" aria-label="Appearance">
                    {(['light', 'dark'] as const).map(theme => <button key={theme} title={`Use ${theme} mode`} disabled={disabled} aria-pressed={prefs?.theme === theme} onClick={() => preference({ theme, backgroundColor: '' })}>{theme === 'light' ? 'Light' : 'Dark'}</button>)}
                  </div>
                  <span className="tasks-label">Photo</span>
                  <div className="tasks-photo-row">
                    <label className={`tasks-file ${disabled ? 'is-disabled' : ''}`} title="Choose a PNG, JPEG, WebP or GIF under 10 MB">
                      <input
                        type="file"
                        accept="image/png,image/jpeg,image/webp,image/gif"
                        disabled={disabled}
                        onChange={(e) => {
                          const file = e.target.files?.[0];
                          e.target.value = '';
                          if (file) void choosePhoto(file);
                        }}
                      />
                      {photo ? 'Change photo' : 'Choose photo'}
                    </label>
                    {photo && (
                      <button title="Remove the photo" disabled={disabled} onClick={() => preference({ backgroundPhoto: null })}>
                        Remove
                      </button>
                    )}
                  </div>
                  {photo && (
                    <>
                      <label className="tasks-field">
                        <span>Fade</span>
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.05"
                          title="How much the photo fades"
                          key={prefs?.fade ?? 0.85}
                          defaultValue={prefs?.fade ?? 0.85}
                          disabled={disabled}
                          onPointerUp={(e) => preference({ fade: Number(e.currentTarget.value) })}
                          onKeyUp={(e) => preference({ fade: Number(e.currentTarget.value) })}
                        />
                      </label>
                      <div className="tasks-segment tasks-wide" role="group" aria-label="Photo position">
                        {(['top', 'center', 'bottom'] as const).map((s) => (
                          <button key={s} title={`Show the ${s} of the photo`} aria-pressed={(prefs?.photoPosition ?? 'center') === s} disabled={disabled} onClick={() => preference({ photoPosition: s })}>
                            {s[0].toUpperCase() + s.slice(1)}
                          </button>
                        ))}
                      </div>
                    </>
                  )}
                </div>
              )}
            </div>
          </div>
        </header>

        {view === 'list' && (
          <form className="tasks-composer" onSubmit={submitComposer}>
            <textarea
              ref={composerRef}
              rows={1}
              value={draft}
              maxLength={20500}
              disabled={disabled}
              aria-label="New task"
              placeholder={innerWidth < 600 ? 'Add a task' : 'Add a task. Enter to add, Shift+Enter for a new line'}
              onChange={(e) => {
                setDraft(e.target.value);
                e.target.style.height = 'auto';
                e.target.style.height = `${e.target.scrollHeight}px`;
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
                  e.preventDefault();
                  void submitComposer();
                  e.currentTarget.style.height = 'auto';
                }
              }}
            />
            <button className="tasks-send" title="Add task (Enter)" aria-label="Add task" disabled={disabled || !draft.trim()}>
              <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
                <path d="M7 11.5V3M3.5 6.5 7 3l3.5 3.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </button>
          </form>
        )}

        {!online && <p className="tasks-note" role="status">Offline. You're seeing your last loaded tasks. Reconnect to edit.</p>}
        {error && (
          <p className="tasks-note is-error" role="alert">
            {error}
          </p>
        )}

        <div className="tasks-body" aria-busy={saving > 0}>
          {!loaded ? <p className="tasks-note" role="status">Loading tasks…</p> : view === 'list' ? listView() : boardView()}
          {loaded && view === 'list' && !showDone && doneCount > 0 && (
            <button
              className="tasks-show-done"
              title="Show done tasks"
              onClick={() => {
                setShowDone(true);
                localStorage.setItem('tasks.showDone', '1');
              }}
            >
              <StatusIcon status="done" /> {doneCount} done
            </button>
          )}
        </div>
      </div>

      {menu && (
        <div className="task-menu" role="menu" style={{ left: menu.x, top: Math.min(menu.y, innerHeight - 330) }}>
          {menu.page === 'main' ? (
            <>
              <button role="menuitem" title="Open the task" onClick={() => openEditor(menu.task)}>
                Open
              </button>
              <button role="menuitem" title="Start a thread from this task" disabled={disabled || !!menu.task.threadCommandId && !menu.task.thread && !menu.task.threadError} onClick={() => void sendTask(menu.task)}>{threadLabel(menu.task)}</button>
              <button role="menuitem" title="Write a note on this task" onClick={() => openEditor(menu.task, true)}>
                {menu.task.notes ? 'Edit note' : 'Add note'}
              </button>
              <button role="menuitem" title="Put this task in a project" onClick={() => setMenu({ ...menu, page: 'project' })}>
                Move to project
                <span className="task-menu-hint">{menu.task.project ? projectName(projectKey(menu.task.project)) : 'None'} ›</span>
              </button>
              <div className="task-menu-sep" />
              <div className="task-menu-statuses" role="group" aria-label="Status">
                {order.map((s) => (
                  <button
                    key={s}
                    role="menuitemradio"
                    aria-checked={membership(menu.task) === s}
                    title={`Move to ${names[s]}`}
                    onClick={() => {
                      void patchTask(menu.task.id, { status: canonical(s), columnId: s });
                      setMenu(null);
                    }}
                  >
                    <StatusIcon status={canonical(s)} />
                    <span>{names[s]}</span>
                  </button>
                ))}
              </div>
              <div className="task-menu-sep" />
              <button
                role="menuitem"
                className="is-danger"
                title="Delete the task. You can undo it."
                onClick={() => {
                  deleteTask(menu.task);
                  setMenu(null);
                }}
              >
                Delete
              </button>
            </>
          ) : (
            <>
              <button role="menuitem" className="task-menu-back" title="Back" onClick={() => setMenu({ ...menu, page: 'main' })}>
                ‹ Move to project
              </button>
              <div className="task-menu-scroll">
                {[{ key: NONE, title: 'No project' }, ...projects.map((p) => ({ key: `${p.runnerId}/${p.id}`, title: p.title }))].map((p) => (
                  <button
                    key={p.key}
                    role="menuitemradio"
                    aria-checked={projectKey(menu.task.project) === p.key}
                    title={`Move to ${p.title}`}
                    onClick={() => {
                      void patchTask(menu.task.id, { project: projectRef(p.key) });
                      setMenu(null);
                    }}
                  >
                    {p.title}
                  </button>
                ))}
              </div>
            </>
          )}
        </div>
      )}

      {toast && (
        <div className="tasks-toast" role="status">
          <span>{toast.text}</span>
          <button
            title="Undo"
            disabled={disabled}
            onClick={() => {
              const undo = toast.undo;
              setToast(null);
              void undo();
            }}
          >
            Undo
          </button>
        </div>
      )}
      {ghost && (
        <div className="task-touch-ghost" style={{ left: ghost.x + 12, top: ghost.y + 12 }}>
          {ghost.label}
        </div>
      )}
      {columnEditor && <div className="task-editor-backdrop"><form className="task-editor" role="dialog" aria-modal="true" aria-label={columnEditor.fresh ? 'Add column' : 'Rename column'} onKeyDown={e => {
        if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); if (!disabled) closeColumnEditor(); }
        if (e.key === 'Tab') {
          const controls = [...e.currentTarget.querySelectorAll<HTMLElement>('input:not(:disabled), button:not(:disabled)')];
          const first = controls[0], last = controls.at(-1);
          if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last?.focus(); }
          if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first?.focus(); }
        }
      }} onSubmit={e => {
        e.preventDefault();
        if (!columnEditor.name.trim() || disabled) return;
        const column = { id: columnEditor.id, name: columnEditor.name.trim(), status: canonical(columnEditor.id) };
        const next = columnEditor.fresh ? [...definitions, column] : definitions.map(c => c.id === column.id ? column : c);
        void write(() => setPrefs({ auth, columns: next })).then(ok => { if (ok) closeColumnEditor(); });
      }}><h2>{columnEditor.fresh ? 'Add column' : 'Rename column'}</h2>{error && <p role="alert">{error}</p>}<input autoFocus aria-label="Column name" maxLength={80} value={columnEditor.name} onChange={e => setColumnEditor({ ...columnEditor, name: e.target.value })} /><footer><button type="button" onClick={() => closeColumnEditor()}>Cancel</button><button disabled={disabled || !columnEditor.name.trim()}>Save</button></footer></form></div>}
      {pendingDelete && (
        <div className="task-editor-backdrop tasks-delete-backdrop">
          <div ref={deleteRef} className="task-editor" role="alertdialog" aria-modal="true" aria-labelledby="task-delete-heading" aria-describedby="task-delete-description" onKeyDown={e => {
            if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); if (!saving) setPendingDelete(null); }
            if (e.key === 'Tab') {
              const buttons = [...e.currentTarget.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];
              const first = buttons[0], last = buttons.at(-1);
              if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last?.focus(); }
              if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first?.focus(); }
            }
          }}>
            <h2 id="task-delete-heading">Delete task?</h2>
            <p id="task-delete-description">Delete “{pendingDelete.title}”? You can undo this afterward.</p>
            {error && <p role="alert">{error}</p>}
            <footer><button type="button" disabled={saving > 0} onClick={() => setPendingDelete(null)}>Cancel</button><button type="button" className="is-danger" disabled={disabled} onClick={() => confirmDelete(pendingDelete)}>Delete</button></footer>
          </div>
        </div>
      )}
      {editing && (
        <div
          className="task-editor-backdrop"
          onMouseDown={(e) => e.target === e.currentTarget && setEditing(null)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') setEditing(null);
          }}
        >
          <form
            ref={editorRef}
            className="task-editor"
            role="dialog"
            aria-modal="true"
            aria-label={editingBaseline.current ? 'Edit task' : 'New task'}
            onSubmit={saveEditor}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                e.currentTarget.requestSubmit();
              }
              if (e.key !== 'Tab') return;
              const fields = Array.from(e.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled)'));
              const first = fields[0];
              const last = fields[fields.length - 1];
              if (e.shiftKey && document.activeElement === first) {
                e.preventDefault();
                last?.focus();
              } else if (!e.shiftKey && document.activeElement === last) {
                e.preventDefault();
                first?.focus();
              }
            }}
          >
            <div className="task-editor-top">
              <span className="task-chip">{projectName(projectKey(editing.project ?? null))}</span>
              <button type="button" className="task-editor-close" title="Close (Esc)" aria-label="Close" onClick={() => setEditing(null)}>
                ×
              </button>
            </div>
            <input className="task-editor-title" aria-label="Title" placeholder="Task title" required maxLength={500} value={editing.title ?? ''} onChange={(e) => setEditing({ ...editing, title: e.target.value })} />
            <textarea className="task-editor-notes" aria-label="Note" placeholder="Add a note…" maxLength={20000} rows={5} value={editing.notes ?? ''} onChange={(e) => setEditing({ ...editing, notes: e.target.value })} />
            <div className="task-editor-chips">
              <label className="task-pill" title="Status">
                <StatusIcon status={editing.status ?? 'todo'} />
                <select aria-label="Status" value={editing.columnId ?? editing.status} onChange={(e) => setEditing({ ...editing, status: canonical(e.target.value), columnId: e.target.value })}>
                  {order.map((s) => (
                    <option key={s} value={s}>
                      {names[s]}
                    </option>
                  ))}
                </select>
              </label>
              <label className="task-pill" title="Project">
                <svg width="13" height="13" viewBox="0 0 14 14" aria-hidden="true">
                  <path d="M7 1.5 12 4v6l-5 2.5L2 10V4zM2 4l5 2.5L12 4M7 6.5v6" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinejoin="round" />
                </svg>
                <select aria-label="Project" value={projectKey(editing.project ?? null)} onChange={(e) => setEditing({ ...editing, project: projectRef(e.target.value) })}>
                  <option value={NONE}>No project</option>
                  {projects.map((p) => (
                    <option key={`${p.runnerId}/${p.id}`} value={`${p.runnerId}/${p.id}`}>
                      {p.title}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <footer>
              {editingBaseline.current && (
                <button
                  type="button"
                  className="is-danger"
                  title="Delete the task. You can undo it."
                  disabled={disabled}
                  onClick={() => {
                    deleteTask(editingBaseline.current!);
                  }}
                >
                  Delete
                </button>
              )}
              <button type="button" title="Save changes and start a thread" disabled={disabled || !editing.title?.trim()} onClick={() => {
                const baseline = editingBaseline.current;
                const live = tasks.find(t => t.id === editing.id) ?? baseline;
                if (live?.thread || live?.threadCommandId) { void sendTask(live); return; }
                if (!editing.id || !editing.title?.trim()) return;
                const id = editing.id;
                const fields = { title: editing.title, notes: editing.notes ?? '', status: editing.status ?? 'todo', columnId: editing.columnId, project: editing.project ?? null };
                void write(async () => {
                  if (baseline) await update({ auth, id, ...taskDraftChanges(baseline, fields) });
                  else await create({ auth, id, ...fields });
                  await startThread({ auth, id });
                }).then(ok => { if (ok) setEditing(null); });
              }}>{threadLabel(tasks.find(t => t.id === editing.id) ?? editingBaseline.current ?? editing)}</button>
              <span className="spacer" />
              <button type="button" title="Close without saving" onClick={() => setEditing(null)}>
                Cancel
              </button>
              <button className="is-primary" title="Save (⌘Enter)" disabled={disabled || !editing.title?.trim()}>
                Save
              </button>
            </footer>
          </form>
        </div>
      )}
    </section>
  );
}

export function TasksPage(props: { token: string; projectFilter?: string }) {
  const me = useQuery(api.phone.me, { token: props.token });
  if (!me) return <p className="pad">Loading Tasks…</p>;
  if (me.apiVersion < 86) return <p className="pad">Update your Threadbear backend to use Tasks here.</p>;
  return <TasksContent key={props.projectFilter ?? ''} {...props} />;
}

export function ProjectTasksLink({ token, runnerId, id }: { token: string; runnerId: string; id: string }) {
  const me = useQuery(api.phone.me, { token });
  const count = useQuery(api.tasks.count, (me?.apiVersion ?? 0) >= 83 ? { auth: { token }, project: { runnerId, id } } : 'skip');
  if ((me?.apiVersion ?? 0) < 83) return null;
  const href = `/tasks/${encodeURIComponent(`${runnerId}/${id}`)}`;
  return (
    <a
      href={href}
      title="Open this project's synced tasks"
      onClick={(e) => {
        if (!e.metaKey && !e.ctrlKey) {
          e.preventDefault();
          go(href);
        }
      }}
    >
      {count === undefined ? 'Tasks' : count.count ? `Tasks ${count.count}${count.more ? '+' : ''}` : 'Add task'}
    </a>
  );
}
