// A thin bridge to cloud-owned Tasks. No second local database or write queue.
import { sniffImage } from './media.ts';
import { readReplyImage } from './share-images.ts';
import { randomUUID } from 'node:crypto';
import path from 'node:path';
import { USER_NAME } from './user-name.ts';
import type { Post, Subscribe, Upload } from './sync.ts';
export type TaskSource = {
  post: Post;
  subscribe?: Subscribe;
  key: string;
  apiVersion?: number;
  connection?: object;
  upload?: Upload;
};
export type TaskStatus = 'backlog' | 'todo' | 'doing' | 'done';
export type NativeTask = {
  id: string;
  title: string;
  notes: string;
  status: TaskStatus;
  columnId?: string;
  threadCommandId?: string;
  thread?: { runnerId: string; id: string };
  threadError?: string;
  project: { runnerId: string; id: string } | null;
  position: number;
  createdAt: number;
  updatedAt: number;
  deleted: boolean;
  // API 87.
  priority?: TaskPriority | null;
  dueDate?: string | null;
  parentId?: string | null;
  links?: TaskLink[];
  commentCount?: number;
  createdBy?: TaskAuthor;
};
export type TaskPriority = 'urgent' | 'high' | 'normal' | 'low';
export type TaskAuthor = { kind: 'user' | 'agent'; name: string };
export type TaskLink = { id: string; kind: 'link' | 'print' | 'image' | 'thread'; title: string; url?: string; printId?: string; thread?: { runnerId: string; id: string }; storageId?: string; imageUrl?: string };
export type TaskPage = { items: NativeTask[]; next: string | null };
// tasks.related: this runner's own threads and prints for a task's project.
export type TaskRelated = {
  threads: { id: string; title: string; status: string; lastActivityAt: string }[];
  prints: { id: string; title: string; kind: string }[];
};
export type TaskLocal = { runnerId: string; related: (projectId: string) => TaskRelated };
const queries = new Set(['list', 'preferences', 'photo', 'projects', 'count', 'activity', 'get']);
const operations = new Set([...queries, 'create', 'update', 'remove', 'restore', 'setPreferences', 'uploadPhoto', 'projectCount', 'photoFile', 'photoData', 'startThread', 'comment', 'removeComment', 'addImage', 'imageData', 'imageFile', 'related']);
// What needs backend API 87: these operations, and these fields anywhere.
const API87_OPERATIONS = new Set(['activity', 'get', 'comment', 'removeComment', 'addImage', 'imageData', 'imageFile']);
const API87_FIELDS = ['priority', 'dueDate', 'parentId', 'links'];
const AUTHORED = new Set(['create', 'update', 'comment']);
const IMAGE_TYPES = ['image/jpeg', 'image/png', 'image/webp', 'image/gif'];
const IMAGE_LIMIT = 10 * 1024 * 1024;
const invalidImage = () => new Error('Choose a PNG, JPEG, WebP or GIF under 10 MB.');
// Bytes from a picker, checked as an image before anything is uploaded.
function imageBytes(base64: unknown): { bytes: Buffer; mediaType: string } {
  if (typeof base64 !== 'string' || base64.length > Math.ceil(IMAGE_LIMIT / 3) * 4 || !/^[A-Za-z0-9+/]*={0,2}$/.test(base64)) throw invalidImage();
  const bytes = Buffer.from(base64, 'base64');
  const image = sniffImage(bytes);
  if (!image || bytes.length > IMAGE_LIMIT || !IMAGE_TYPES.includes(image.mediaType)) throw invalidImage();
  return { bytes, mediaType: image.mediaType };
}
function imageFile(file: unknown): { bytes: Uint8Array; mediaType: string } {
  if (typeof file !== 'string') throw new Error('Choose a photo on this Mac.');
  const image = readReplyImage(file);
  if ('skip' in image || image.bytes.byteLength > IMAGE_LIMIT || !IMAGE_TYPES.includes(image.mediaType)) throw invalidImage();
  return image;
}
// The newest ten of each, from what this runner keeps for the project.
export function relatedFor(
  deps: {
    threadIds: (projectId: string) => string[] | undefined;
    thread: (id: string) => { id: string; title: string; status: string; lastActivityAt: string } | undefined;
    prints: (projectId: string) => { id: string; title: string; kind?: string; updatedAt: string }[];
  },
  projectId: string,
): TaskRelated {
  const threads = (deps.threadIds(projectId) ?? [])
    .flatMap((id) => {
      const t = deps.thread(id);
      return t ? [{ id: t.id, title: t.title, status: t.status, lastActivityAt: t.lastActivityAt }] : [];
    })
    .sort((a, b) => b.lastActivityAt.localeCompare(a.lastActivityAt))
    .slice(0, 10);
  const prints = [...deps.prints(projectId)]
    .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
    .slice(0, 10)
    .map((p) => ({ id: p.id, title: p.title, kind: p.kind ?? 'page' }));
  return { threads, prints };
}
export class Tasks {
  private stopWatch?: () => void;
  private watched?: { key: string; connection?: object };
  private source: () => TaskSource | undefined;
  private enabled: () => boolean;
  private changed: () => void;
  private local: () => TaskLocal | undefined;
  constructor(source: () => TaskSource | undefined, enabled: () => boolean, changed: () => void, local: () => TaskLocal | undefined = () => undefined) {
    this.source = source;
    this.enabled = enabled;
    this.changed = changed;
    this.local = local;
  }
  // author: who a create, update or comment is from. Left out, the user (the
  // Mac app); agents' tools pass their own. Never taken from args.
  async call(operation: string, args: Record<string, unknown> = {}, author?: TaskAuthor): Promise<unknown> {
    if (!this.enabled()) {
      this.stop();
      throw new Error('Turn on Tasks in Settings > Experiments.');
    }
    if (!operations.has(operation)) throw new Error('Unknown Tasks operation');
    const source = this.source();
    if (!source) {
      this.stop();
      throw new Error('Tasks needs a connection to your Threadbear backend. Your draft has not been saved.');
    }
    if ((source.apiVersion ?? 84) < 84) throw new Error('Update the Threadbear backend to use Tasks.');
    if (source.apiVersion !== undefined && source.apiVersion < 86 && (operation === 'startThread' || ['columnId', 'columns', 'boardColumnOrder', 'sectionOrder'].some(field => field in args)))
      throw new Error('Update the Threadbear backend to use task columns and Send to thread.');
    const api87 = source.apiVersion === undefined || source.apiVersion >= 87;
    if (!api87 && (API87_OPERATIONS.has(operation) || API87_FIELDS.some((field) => field in args)))
      throw new Error('Update the Threadbear backend to use task priority, due dates, sub-tasks, links and comments.');
    if (operation === 'related') {
      const local = this.local();
      const empty: TaskRelated = { threads: [], prints: [] };
      if (!local || typeof args.id !== 'string') return empty;
      let project = args.project as NativeTask['project'] | undefined;
      if (project === undefined) {
        if (!api87) return empty;
        project = ((await this.call('get', { id: args.id })) as NativeTask | null)?.project;
      }
      return project && project.runnerId === local.runnerId ? local.related(project.id) : empty;
    }
    if (operation === 'photoData' || operation === 'photoFile') {
      const image = operation === 'photoData' ? imageBytes(args.base64) : imageFile(args.path);
      if (!source.upload) throw new Error('Photo uploads are unavailable on this runner.');
      const url = (await this.call('uploadPhoto')) as string;
      const storageId = await source.upload(url, image.bytes, image.mediaType);
      return this.call('setPreferences', { backgroundPhoto: storageId });
    }
    if (operation === 'imageData' || operation === 'imageFile') {
      if (typeof args.id !== 'string') throw new Error('Choose a task for the image.');
      const image = operation === 'imageData' ? imageBytes(args.base64) : imageFile(args.path);
      if (!source.upload) throw new Error('Image uploads are unavailable on this runner.');
      const name = operation === 'imageFile' ? path.basename(args.path as string) : typeof args.name === 'string' ? args.name : undefined;
      const url = (await this.call('uploadPhoto')) as string;
      const storageId = await source.upload(url, image.bytes, image.mediaType);
      return this.call('addImage', { id: args.id, storageId, ...(name ? { title: name.slice(0, 300) } : {}) });
    }
    // Never accept credentials or an author from a UI or agent; this runner supplies its own.
    const { auth: _auth, author: _author, ...fields } = args;
    // Only to a backend known to take it: an older one refuses the whole write.
    if (source.apiVersion !== undefined && source.apiVersion >= 87 && AUTHORED.has(operation)) fields.author = author ?? { kind: 'user', name: USER_NAME ?? 'You' };
    if (typeof fields.project === 'string') {
      const options = (await this.call('projects')) as {
        id: string;
        runnerId: string;
        title: string;
      }[];
      const query = fields.project.toLowerCase();
      const matches = options.filter((p) => p.id === fields.project || p.title.toLowerCase() === query);
      if (matches.length !== 1) throw new Error(matches.length ? 'Several projects match. Choose its runnerId and id from list_tasks.' : 'No matching project. list_tasks includes project choices.');
      fields.project = { id: matches[0]!.id, runnerId: matches[0]!.runnerId };
    }
    if (operation === 'projectCount') return this.call('count', fields);
    if (!this.stopWatch || this.watched?.key !== source.key || this.watched?.connection !== source.connection) this.watch();
    const result = await source.post(queries.has(operation) ? 'query' : 'mutation', `tasks:${operation}`, {
      ...fields,
      ...(operation === 'create' && !fields.id ? { id: randomUUID() } : {}),
      auth: { key: source.key },
    });
    if (!queries.has(operation)) this.changed();
    return result;
  }
  watch(): void {
    this.stop();
    if (!this.enabled()) return;
    const source = this.source();
    if (!source?.subscribe) return;
    this.watched = { key: source.key, connection: source.connection };
    const stops = ['revision', 'preferences', 'projects'].map((name) =>
      source.subscribe!(
        `tasks:${name}`,
        { auth: { key: source.key } },
        () => this.changed(),
        () => this.changed(),
      ),
    );
    this.stopWatch = () => stops.forEach((stop) => stop());
  }
  stop(): void {
    this.stopWatch?.();
    this.stopWatch = undefined;
    this.watched = undefined;
  }
}
export function taskMethods(tasks: Tasks): Record<string, (p: Record<string, unknown>) => unknown> {
  return Object.fromEntries([...operations].map((operation) => [`tasks.${operation}`, (p: Record<string, unknown>) => tasks.call(operation, p)]));
}
