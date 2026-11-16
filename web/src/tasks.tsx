import { useEffect, useState, useRef, type CSSProperties, type FormEvent } from 'react';
import { useConvex, useConvexConnectionState, useMutation, useQuery } from 'convex/react';
import type { FunctionReturnType } from 'convex/server';
import type { Id } from '../../convex/_generated/dataModel';
import { api } from '../../convex/_generated/api';
import './tasks.css';
import { go } from './route';
import { applyTheme, readTheme } from './theme';
import { taskDraftChanges } from './tasks-draft';

type Task = FunctionReturnType<typeof api.tasks.list>['items'][number];
type Status = Task['status'];
type Prefs = FunctionReturnType<typeof api.tasks.preferences>;
const statuses: Status[] = ['backlog', 'todo', 'done'];
const names = { backlog: 'Backlog', todo: 'Todo', done: 'Done' };
const projectKey = (p: Task['project']) => (p ? `${p.runnerId}/${p.id}` : '');

function TasksContent({ token, projectFilter = '' }: { token: string; projectFilter?: string }) {
  const auth = { token };
  const convex = useConvex();
  const connection = useConvexConnectionState();
  const revision = useQuery(api.tasks.revision, { auth });
  const prefs = useQuery(api.tasks.preferences, { auth });
  const photo = useQuery(api.tasks.photo, { auth });
  const projects = useQuery(api.phone.projects, { token }) ?? [];
  const projectIds = projects
    .map((p) => `${p.runnerId}/${p.id}`)
    .sort()
    .join('|');
  const create = useMutation(api.tasks.create);
  const update = useMutation(api.tasks.update);
  const remove = useMutation(api.tasks.remove);
  const restore = useMutation(api.tasks.restore);
  const setPrefs = useMutation(api.tasks.setPreferences);
  const upload = useMutation(api.tasks.uploadPhoto);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [currentView, setView] = useState<'list' | 'board' | null>(null);
  const [filter, setFilter] = useState(projectFilter);
  const editingBaseline = useRef<Task | null>(null);
  const [editing, setEditing] = useState<Partial<Task> | null>(null);
  const [customize, setCustomize] = useState(false);
  const [undo, setUndo] = useState<string | null>(null);
  const busyRef = useRef(false);
  const editorRef = useRef<HTMLFormElement>(null);
  const [touchGhost, setTouchGhost] = useState<{
    x: number;
    y: number;
    label: string;
  } | null>(null);
  const [drag, setDrag] = useState<{ task?: string; column?: Status } | null>(null);
  const view = currentView ?? prefs?.defaultView ?? 'board';
  const online = connection.isWebSocketConnected;
  const disabled = busy || !online;
  const order = prefs?.columnOrder ?? statuses;
  useEffect(() => {
    if (!prefs?.theme) return;
    applyTheme(prefs.theme);
    return () => applyTheme(readTheme());
  }, [prefs?.theme]);
  useEffect(() => {
    if (!editing) return;
    const previous = document.activeElement as HTMLElement | null;
    editorRef.current?.querySelector<HTMLInputElement>('input')?.focus();
    return () => previous?.focus();
  }, [editing?.id]);
  // Reconcile complete pages on an external realtime invalidation, retaining the
  // last successful snapshot while disconnected. Never persist tokens or notes.
  useEffect(() => {
    if (revision === undefined) return;
    let cancelled = false;
    void (async () => {
      const all: Task[] = [];
      let after: string | undefined;
      do {
        const page = await convex.query(api.tasks.list, {
          auth: { token },
          after,
          limit: 250,
        });
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
  async function run(work: () => Promise<unknown>): Promise<boolean> {
    if (!online || busyRef.current) return false;
    busyRef.current = true;
    setBusy(true);
    setError('');
    try {
      await work();
      return true;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return false;
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }
  const preference = (patch: Partial<Prefs>) => void run(() => setPrefs({ auth, ...patch }));
  const projectName = (key: string) => projects.find((p) => `${p.runnerId}/${p.id}` === key)?.title ?? 'No project';
  const visible = tasks.filter((t) => !filter || (filter === 'none' ? !t.project : projectKey(t.project) === filter));
  visible.sort((a, b) =>
    prefs?.sort === 'title'
      ? a.title.localeCompare(b.title)
      : prefs?.sort === 'newest'
        ? b.createdAt - a.createdAt
        : prefs?.sort === 'oldest'
          ? a.createdAt - b.createdAt
          : a.position - b.position || a.id.localeCompare(b.id),
  );
  const groups = prefs?.grouping === 'project' ? [...new Set(visible.map((t) => projectKey(t.project)))] : ['all'];
  const add = (status: Status = 'todo', group = filter) => {
    editingBaseline.current = null;
    setEditing({
      id: crypto.randomUUID(),
      title: '',
      notes: '',
      status,
      project: projects.find((p) => `${p.runnerId}/${p.id}` === (group === 'all' ? filter : group)) ?? null,
    });
  };
  async function save(e: FormEvent) {
    e.preventDefault();
    if (!editing?.id || !editing.title?.trim()) return;
    const { id, title = '', notes = '', status = 'todo', project = null } = editing;
    const patch = {
      id,
      title,
      notes,
      status,
      project: project ? { id: project.id, runnerId: project.runnerId } : null,
    };
    const base = editingBaseline.current;
    if (await run(() => (base ? update({ auth, id, ...taskDraftChanges(base, patch) }) : create({ auth, ...patch })))) setEditing(null);
  }
  function drop(status: Status, before?: Task, group?: string) {
    if (!drag || disabled) return;
    if (drag.column) {
      if (drag.column === status) {
        setDrag(null);
        return;
      }
      const next = order.filter((s) => s !== drag.column);
      next.splice(next.indexOf(status), 0, drag.column);
      preference({ columnOrder: next });
    } else if (drag.task) {
      if (before?.id === drag.task) {
        setDrag(null);
        return;
      }
      const target = visible.filter((t) => t.status === status && t.id !== drag.task && (group === undefined || group === 'all' || projectKey(t.project) === group));
      const index = before ? target.findIndex((t) => t.id === before.id) : target.length;
      const high = target[index]?.position;
      const low = target[index - 1]?.position;
      const position = high === undefined ? (low ?? Date.now()) + 1000 : low === undefined ? high - 1000 : (low + high) / 2;
      const project = group !== undefined && group !== 'all' ? projects.find((p) => `${p.runnerId}/${p.id}` === group) : undefined;
      void run(() =>
        update({
          auth,
          id: drag.task!,
          status,
          position,
          ...(group !== undefined && group !== 'all'
            ? {
                project: project ? { id: project.id, runnerId: project.runnerId } : null,
              }
            : {}),
        }),
      );
    }
    setDrag(null);
  }
  function finishTouch(e: React.PointerEvent<HTMLElement>) {
    if (!touchGhost) return;
    const target = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-task-status]');
    if (target)
      drop(
        target.dataset.taskStatus as Status,
        tasks.find((t) => t.id === target.dataset.taskId),
        target.dataset.taskProject,
      );
    setTouchGhost(null);
    setDrag(null);
  }
  function moveByKey(t: Task, direction: number) {
    const items = visible.filter((r) => r.status === t.status && (prefs?.grouping !== 'project' || projectKey(r.project) === projectKey(t.project)));
    const index = items.findIndex((r) => r.id === t.id);
    const target = index + direction;
    if (target < 0 || target >= items.length) return;
    const neighbor = items[target];
    const other = items[target + direction];
    const position = other ? (neighbor.position + other.position) / 2 : neighbor.position + direction * 1000;
    void run(() => update({ auth, id: t.id, position }));
  }
  function card(t: Task) {
    return (
      <article
        data-task-status={t.status}
        data-task-id={t.id}
        data-task-project={prefs?.grouping === 'project' ? projectKey(t.project) : 'all'}
        key={t.id}
        className={`task-card ${t.status === 'done' ? 'is-done' : ''}`}
        draggable={!disabled && (prefs?.sort ?? 'manual') === 'manual'}
        onDragStart={(e) => {
          e.stopPropagation();
          setDrag({ task: t.id });
        }}
        onDragEnd={() => setDrag(null)}
        onDragOver={(e) => e.preventDefault()}
        onDrop={(e) => {
          e.preventDefault();
          e.stopPropagation();
          drop(t.status, t, prefs?.grouping === 'project' ? projectKey(t.project) : 'all');
        }}
      >
        <button
          className="task-drag"
          title="Drag to move; arrow keys reorder"
          aria-label={`Move ${t.title}`}
          disabled={disabled || (prefs?.sort ?? 'manual') !== 'manual'}
          onPointerDown={(e) => {
            if (e.pointerType === 'mouse') return;
            e.currentTarget.setPointerCapture(e.pointerId);
            setDrag({ task: t.id });
            setTouchGhost({ x: e.clientX, y: e.clientY, label: t.title });
          }}
          onPointerMove={(e) => {
            if (touchGhost) setTouchGhost({ x: e.clientX, y: e.clientY, label: t.title });
          }}
          onPointerUp={finishTouch}
          onPointerCancel={() => {
            setTouchGhost(null);
            setDrag(null);
          }}
          onKeyDown={(e) => {
            if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
              e.preventDefault();
              moveByKey(t, e.key === 'ArrowUp' ? -1 : 1);
            }
          }}
        >
          ⋮⋮
        </button>
        <input
          type="checkbox"
          title="Mark task done"
          aria-label={`Mark ${t.title} ${t.status === 'done' ? 'todo' : 'done'}`}
          checked={t.status === 'done'}
          disabled={disabled}
          onChange={() =>
            void run(() =>
              update({
                auth,
                id: t.id,
                status: t.status === 'done' ? 'todo' : 'done',
              }),
            )
          }
        />
        <button
          className="task-title"
          title="Edit task"
          onClick={() => {
            editingBaseline.current = t;
            setEditing(t);
          }}
        >
          {t.title}
        </button>
        {t.project && <span className="task-project">{projectName(projectKey(t.project))}</span>}
        <select title="Move task" aria-label={`Status of ${t.title}`} value={t.status} disabled={disabled} onChange={(e) => void run(() => update({ auth, id: t.id, status: e.target.value as Status }))}>
          {statuses.map((s) => (
            <option key={s} value={s}>
              {names[s]}
            </option>
          ))}
        </select>
      </article>
    );
  }
  const background = {
    '--tasks-color': prefs?.backgroundColor || 'var(--bg, #18191c)',
    '--tasks-photo': photo ? `url(${JSON.stringify(photo)})` : 'none',
    '--tasks-fade': prefs?.fade ?? 0.85,
    '--tasks-position': prefs?.photoPosition ?? 'center',
  } as CSSProperties;
  return (
    <section className={`tasks-page tasks-${view}`} style={background} aria-label="Tasks">
      <header className="tasks-header">
        <h1>Tasks</h1>
        <div className="tasks-view" aria-label="Task view">
          {(['list', 'board'] as const).map((v) => (
            <button key={v} title={`Show ${v}`} aria-pressed={view === v} onClick={() => setView(v)}>
              {v === 'list' ? 'List' : 'Board'}
            </button>
          ))}
          <button title="Set this view as the default" disabled={disabled || view === prefs?.defaultView} onClick={() => preference({ defaultView: view })}>
            {view === prefs?.defaultView ? 'Default' : 'Set as default'}
          </button>
        </div>
        <button title="Customize Tasks background" onClick={() => setCustomize(!customize)}>
          Customize
        </button>
        <button title="Add task" disabled={disabled} onClick={() => add()}>
          Add task
        </button>
      </header>
      <div className="tasks-toolbar">
        <select aria-label="Filter project" title="Filter project" value={filter} onChange={(e) => setFilter(e.target.value)}>
          <option value="">All projects</option>
          <option value="none">No project</option>
          {projects.map((p) => (
            <option key={`${p.runnerId}/${p.id}`} value={`${p.runnerId}/${p.id}`}>
              {p.title}
            </option>
          ))}
        </select>
        <select aria-label="Group tasks" title="Group tasks" value={prefs?.grouping ?? 'status'} disabled={disabled} onChange={(e) => preference({ grouping: e.target.value as Prefs['grouping'] })}>
          <option value="status">Group by status</option>
          <option value="project">Group by project</option>
        </select>
        <select aria-label="Sort tasks" title="Sort tasks" value={prefs?.sort ?? 'manual'} disabled={disabled} onChange={(e) => preference({ sort: e.target.value as Prefs['sort'] })}>
          {['manual', 'newest', 'oldest', 'title'].map((s) => (
            <option key={s} value={s}>
              {s[0].toUpperCase() + s.slice(1)}
            </option>
          ))}
        </select>
        <span>{visible.length} {visible.length === 1 ? 'task' : 'tasks'}</span>
      </div>
      {!online && <p role="status">Offline. Your last loaded tasks are shown. Reconnect to edit.</p>}
      {error && <p role="alert">{error}</p>}
      {busy && <p role="status">Saving…</p>}
      {undo && (
        <p role="status">
          Task deleted.{' '}
          <button
            title="Undo deletion"
            disabled={disabled}
            onClick={() =>
              void run(() => restore({ auth, id: undo })).then((ok) => {
                if (ok) setUndo(null);
              })
            }
          >
            Undo
          </button>
        </p>
      )}
      {customize && (
        <div className="tasks-customize">
          <label>
            Background color <input type="color" title="Background color" value={prefs?.backgroundColor || '#18191c'} disabled={disabled} onChange={(e) => preference({ backgroundColor: e.target.value })} />
          </label>
          <button title="Use theme background" disabled={disabled} onClick={() => preference({ backgroundColor: '', backgroundPhoto: null })}>
            Use theme background
          </button>
          <label>
            Photo{' '}
            <input
              type="file"
              accept="image/png,image/jpeg,image/webp,image/gif"
              disabled={disabled}
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (!file) return;
                void run(async () => {
                  if (file.size > 10 * 1024 * 1024) throw new Error('Choose a photo under 10 MB');
                  const url = await upload({ auth });
                  const response = await fetch(url, {
                    method: 'POST',
                    body: file,
                    headers: { 'Content-Type': file.type },
                  });
                  if (!response.ok) throw new Error('Photo upload failed. Try again.');
                  const { storageId } = (await response.json()) as {
                    storageId: Id<'_storage'>;
                  };
                  await setPrefs({ auth, backgroundPhoto: storageId });
                });
              }}
            />
          </label>
          <label>
            Fade{' '}
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              key={prefs?.fade ?? 0.85}
              defaultValue={prefs?.fade ?? 0.85}
              disabled={disabled}
              onPointerUp={(e) => preference({ fade: Number(e.currentTarget.value) })}
              onKeyUp={(e) => preference({ fade: Number(e.currentTarget.value) })}
            />
          </label>
          <select
            title="Photo position"
            aria-label="Photo position"
            value={prefs?.photoPosition ?? 'center'}
            disabled={disabled}
            onChange={(e) =>
              preference({
                photoPosition: e.target.value as Prefs['photoPosition'],
              })
            }
          >
            {['top', 'center', 'bottom'].map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </div>
      )}
      {!loaded ? (
        <p role="status">Loading tasks…</p>
      ) : (
        (groups.length ? groups : ['all']).map((group) => (
          <div className="tasks-group" key={group}>
            {group !== 'all' && <h2>{projectName(group)}</h2>}
            <div className="tasks-columns">
              {order.map((status) => {
                const items = visible.filter((t) => t.status === status && (group === 'all' || projectKey(t.project) === group));
                return (
                  <section
                    data-task-status={status}
                    data-task-project={group}
                    className="task-column"
                    key={status}
                    onDragOver={(e) => e.preventDefault()}
                    onDrop={(e) => {
                      e.preventDefault();
                      drop(status, undefined, group);
                    }}
                  >
                    <h2
                      tabIndex={0}
                      title="Drag to reorder column; arrow keys move it"
                      onPointerDown={(e) => {
                        if (e.pointerType === 'mouse' || disabled || (e.target as HTMLElement).closest('button')) return;
                        e.currentTarget.setPointerCapture(e.pointerId);
                        setDrag({ column: status });
                        setTouchGhost({ x: e.clientX, y: e.clientY, label: names[status] });
                      }}
                      onPointerMove={(e) => {
                        if (touchGhost && drag?.column) setTouchGhost({ x: e.clientX, y: e.clientY, label: names[status] });
                      }}
                      onPointerUp={finishTouch}
                      onPointerCancel={() => {
                        setTouchGhost(null);
                        setDrag(null);
                      }}
                      onKeyDown={(e) => {
                        if (!['ArrowLeft', 'ArrowRight'].includes(e.key)) return;
                        e.preventDefault();
                        const next = [...order],
                          index = next.indexOf(status),
                          target = index + (e.key === 'ArrowLeft' ? -1 : 1);
                        if (target < 0 || target >= next.length) return;
                        [next[index], next[target]] = [next[target], next[index]];
                        preference({ columnOrder: next });
                      }}
                      draggable={!disabled}
                      onDragStart={() => setDrag({ column: status })}
                      onDragEnd={() => setDrag(null)}
                    >
                      {names[status]} <span>{items.length}</span>
                      <button title={`Add to ${names[status]}`} aria-label={`Add to ${names[status]}`} disabled={disabled} onClick={() => add(status, group)}>
                        +
                      </button>
                    </h2>
                    {items.map(card)}
                    <button className="task-add" title={`Add task to ${names[status]}`} disabled={disabled} onClick={() => add(status, group)}>
                      Add task
                    </button>
                  </section>
                );
              })}
            </div>
          </div>
        ))
      )}
      {touchGhost && (
        <div className="task-touch-ghost" style={{ left: touchGhost.x + 12, top: touchGhost.y + 12 }}>
          {touchGhost.label}
        </div>
      )}
      {editing && (
        <div
          className="task-editor-backdrop"
          onKeyDown={(e) => {
            if (e.key === 'Escape' && !busy) setEditing(null);
          }}
        >
          <form
            ref={editorRef}
            onKeyDown={(e) => {
              if (e.key !== 'Tab') return;
              const fields = Array.from(e.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled)'));
              const first = fields[0],
                last = fields[fields.length - 1];
              if (e.shiftKey && document.activeElement === first) {
                e.preventDefault();
                last?.focus();
              } else if (!e.shiftKey && document.activeElement === last) {
                e.preventDefault();
                first?.focus();
              }
            }}
            className="task-editor"
            role="dialog"
            aria-modal="true"
            aria-label="Edit task"
            onSubmit={save}
          >
            <h2>{tasks.some((t) => t.id === editing.id) ? 'Edit task' : 'Add task'}</h2>
            <p role="alert">{error}</p>
            <label>
              Title
              <input autoFocus disabled={busy} required maxLength={500} value={editing.title ?? ''} onChange={(e) => setEditing({ ...editing, title: e.target.value })} />
            </label>
            <label>
              Notes
              <textarea disabled={busy} maxLength={20000} rows={6} value={editing.notes ?? ''} onChange={(e) => setEditing({ ...editing, notes: e.target.value })} />
            </label>
            <label>
              Status
              <select disabled={busy} value={editing.status} onChange={(e) => setEditing({ ...editing, status: e.target.value as Status })}>
                {statuses.map((s) => (
                  <option key={s} value={s}>
                    {names[s]}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Project
              <select
                disabled={busy}
                value={projectKey(editing.project ?? null)}
                onChange={(e) => {
                  const p = projects.find((p) => `${p.runnerId}/${p.id}` === e.target.value);
                  setEditing({
                    ...editing,
                    project: p ? { runnerId: p.runnerId, id: p.id } : null,
                  });
                }}
              >
                <option value="">No project</option>
                {projects.map((p) => (
                  <option key={`${p.runnerId}/${p.id}`} value={`${p.runnerId}/${p.id}`}>
                    {p.title}
                  </option>
                ))}
              </select>
            </label>
            <footer>
              <button type="button" title="Cancel edit" disabled={busy} onClick={() => setEditing(null)}>
                Cancel
              </button>
              {tasks.some((t) => t.id === editing.id) && (
                <button
                  type="button"
                  title="Delete task"
                  disabled={disabled}
                  onClick={() =>
                    void run(() => remove({ auth, id: editing.id! })).then((ok) => {
                      if (ok) {
                        setUndo(editing.id!);
                        setEditing(null);
                      }
                    })
                  }
                >
                  Delete
                </button>
              )}
              <button title="Save task" disabled={disabled || !editing.title?.trim()}>
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
  if (me.apiVersion < 83) return <p className="pad">Update your Threadbear backend to use Tasks here.</p>;
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
