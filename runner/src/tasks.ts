// A thin bridge to cloud-owned Tasks. No second local database or write queue.
import { readReplyImage } from './share-images.ts';
import { randomUUID } from 'node:crypto';
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
  project: { runnerId: string; id: string } | null;
  position: number;
  createdAt: number;
  updatedAt: number;
  deleted: boolean;
};
export type TaskPage = { items: NativeTask[]; next: string | null };
const queries = new Set(['list', 'preferences', 'photo', 'projects', 'count']);
const operations = new Set([...queries, 'create', 'update', 'remove', 'restore', 'setPreferences', 'uploadPhoto', 'projectCount', 'photoFile']);
export class Tasks {
  private stopWatch?: () => void;
  private watched?: { key: string; connection?: object };
  private source: () => TaskSource | undefined;
  private enabled: () => boolean;
  private changed: () => void;
  constructor(source: () => TaskSource | undefined, enabled: () => boolean, changed: () => void) {
    this.source = source;
    this.enabled = enabled;
    this.changed = changed;
  }
  async call(operation: string, args: Record<string, unknown> = {}): Promise<unknown> {
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
    if (operation === 'photoFile') {
      if (typeof args.path !== 'string' || !source.upload) throw new Error('Choose a photo on this Mac.');
      const image = readReplyImage(args.path);
      if ('skip' in image || image.bytes.byteLength > 10 * 1024 * 1024 || !['image/jpeg', 'image/png', 'image/webp', 'image/gif'].includes(image.mediaType))
        throw new Error('Choose a PNG, JPEG, WebP or GIF under 10 MB.');
      const url = (await this.call('uploadPhoto')) as string;
      const storageId = await source.upload(url, image.bytes, image.mediaType);
      return this.call('setPreferences', { backgroundPhoto: storageId });
    }
    // Never accept credentials from a UI or agent; this runner supplies its own.
    const { auth: _auth, ...fields } = args;
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
