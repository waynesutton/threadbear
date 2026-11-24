// Setting up a Fly virtual runner from the Mac (docs/virtual-runners.md,
// "Setting one up"; docs/protocol/keys.md, "Cloud setup"): one Fly token in,
// or flyctl's own sign-in, and every step the forking notes list by hand
// done here. Each step checks first and does only what's missing, so a
// second run changes nothing, and the same steps answer Check again.
//
// What it writes is what Mike's backend already reads: FLY_API_TOKEN,
// FLY_APP and FLY_ORG as Convex env vars (convex/boxProviders/fly.ts
// flyFromEnv), BOX_SEAL_KEY only when it isn't set (a new one would make
// every sealed cloud token unreadable, convex/boxes.ts), the box image at
// registry.fly.io/<app>:latest (boxes.ts defaultBoxImage), and a
// runnerServices row through boxes:setRunnerService. Nothing in convex/
// changes for it.
//
// The Fly token goes to the backend's env, to flyctl's environment for the
// build, and nowhere else: not to disk, the log or an error. It's handed to
// the transcript scrubber the moment it's seen (rememberToken).
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Logger } from './log.ts';
import { ErrorCode, RpcError } from './protocol.ts';
import { rememberToken } from './year-token.ts';
import { estimate, type CloudCost, type FlyMachine } from './fly-cost.ts';

export const FLY_API = 'https://api.machines.dev/v1';

export type Run = { ok: boolean; out: string; err: string; code?: number | null };

export type StepId = 'flyctl' | 'token' | 'app' | 'backend' | 'image' | 'updates' | 'service' | 'test' | 'archived' | 'unlist' | 'unset' | 'flyApp';
export type StepState = 'todo' | 'doing' | 'ok' | 'warn' | 'fail' | 'skipped';
export type Step = { id: StepId; title: string; state: StepState; detail?: string };

export type SetupParams = {
  // A pasted token, or none to mint one with flyctl's own sign-in (`fly auth login`).
  token?: string;
  useFlyctl?: boolean;
  org?: string;
  app?: string;
  name?: string;
  serviceId?: string;
  cpus?: number;
  memoryMb?: number;
};

export type Service = { serviceId: string; name: string; provider: string; image: string; box: { cpus: number; memoryMb: number }; running: number; asleep: number };

export type CloudStatus = {
  // The last setup, check or rebuild this runner ran, or is running.
  job?: { kind: 'setup' | 'check' | 'rebuild' | 'remove'; running: boolean; steps: Step[]; startedAt: number; endedAt?: number; error?: string };
  services: Service[];
  // The last cloud.cost this runner worked out, if any since it started.
  cost?: CloudCost;
};

export type CloudDeps = {
  // The virtualRunners experiment on, a backend of API 94 and the owner key.
  on(): boolean;
  call(kind: 'action' | 'query' | 'mutation', fn: string, args: Record<string, unknown>): Promise<unknown>;
  // The Convex CLI on this Mac's deploy key (runner-keys.ts convexCli): env get, set.
  convex(args: string[], input?: string): Run;
  // flyctl and gh. The token comes in env, never in args.
  exec(cmd: string, args: string[], opts?: { env?: NodeJS.ProcessEnv; input?: string; cwd?: string; timeoutMs?: number }): Promise<Run>;
  fetch(url: string, init?: RequestInit): Promise<Response>;
  // The Threadbear checkout the runner came from (release.ts ownCheckout): the image builds from its runner/.
  checkout(): string;
  log: Logger;
  now(): number;
  flyApi?: string;
  sleep?(ms: number): Promise<void>;
};

const TITLES: Record<StepId, string> = {
  flyctl: 'flyctl installed',
  token: 'Fly token works',
  app: 'App for the boxes',
  backend: 'Token and keys saved to your backend',
  image: 'Box image',
  updates: 'Image follows main (GitHub)',
  service: 'Cloud runner',
  test: 'Test box: start, check, delete',
  archived: 'Its threads are archived',
  unlist: 'Cloud runner removed',
  unset: 'Token and app cleared from your backend',
  flyApp: 'Fly app',
};

export type RemoveParams = { serviceId?: string; deleteApp?: boolean };

const SETUP: StepId[] = ['flyctl', 'token', 'app', 'backend', 'image', 'updates', 'service', 'test'];
const CHECK: StepId[] = ['flyctl', 'token', 'app', 'backend', 'service'];

// Fly's names: lower case letters, digits and dashes.
const APP_NAME = /^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$/;
const ORG_SLUG = /^[a-z0-9][a-z0-9-]{0,62}$/;
// A token from `fly tokens create` (FlyV1 fm2_…) or an older personal one (fo1_…).
const FLY_TOKEN = /^(?:FlyV1 [A-Za-z0-9_+/=,.-]{20,}|fo1_[A-Za-z0-9_-]{20,})$/;

const MB_STEP = 256;
const FLY_SUSPEND_MAX_MB = 2048;
// The image build: Claude Code's download and npm ci on Fly's remote builder.
const BUILD_MS = 20 * 60_000;
// The smoke box: no boot token, so the runner says why and exits 64 (runner/box/smoke.sh).
const NO_BOOT_EXIT = 64;

const bad = (m: string) => new RpcError(ErrorCode.badParams, m);

export function cleanToken(raw: string): string {
  // A paste keeps `FlyV1 ` and loses any newlines and surrounding space.
  const t = raw.replace(/[\r\n]+/g, '').trim();
  return t.startsWith('FlyV1') ? `FlyV1 ${t.slice(5).trim()}` : t.replace(/\s+/g, '');
}

export function looksLikeFlyToken(t: string): boolean {
  return FLY_TOKEN.test(t);
}

/** The default app name: tb-boxes- and something of this Mac's user, made safe for Fly. */
export function defaultAppName(user = os.userInfo().username): string {
  const tail = user.toLowerCase().replace(/[^a-z0-9-]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 40) || 'me';
  return `tb-boxes-${tail}`;
}

// Hide a token wherever it might show: in flyctl's or Fly's own words.
function scrub(s: string, token: string | undefined): string {
  let out = s;
  if (token) {
    out = out.split(token).join('[REDACTED]');
    const bare = token.replace(/^FlyV1 /, '');
    if (bare.length > 12) out = out.split(bare).join('[REDACTED]');
  }
  return out.replace(/FlyV1 [A-Za-z0-9_+/=,.-]{20,}/g, 'FlyV1 [REDACTED]').replace(/\bfo1_[A-Za-z0-9_-]{20,}/g, 'fo1_[REDACTED]');
}

function lastLine(s: string): string {
  const lines = s.trim().split('\n').map((l) => l.trim()).filter(Boolean);
  return (lines[lines.length - 1] ?? '').slice(0, 300);
}

// The token flyctl prints, whole: an org token runs to hundreds of characters,
// so it mustn't go through lastLine's cut.
export function tokenLine(s: string): string {
  const lines = s.trim().split('\n').map((l) => l.trim()).filter(Boolean);
  return lines[lines.length - 1] ?? '';
}

export class CloudSetup {
  private job: CloudStatus['job'];
  private lastCost: CloudCost | undefined;
  private going: Promise<void> | undefined;
  private deps: CloudDeps;
  constructor(deps: CloudDeps) {
    this.deps = deps;
  }

  private mustBeOn(): void {
    if (!this.deps.on()) throw bad("Cloud runners aren't on for this runner (the virtualRunners experiment, a backend of API 94 and the main Mac's key).");
  }

  private api(): string {
    return (this.deps.flyApi ?? FLY_API).replace(/\/$/, '');
  }

  private async fly<T>(token: string, method: string, p: string, body?: unknown): Promise<{ status: number; json?: T; text: string }> {
    const res = await this.deps.fetch(`${this.api()}${p}`, {
      method,
      headers: { authorization: `Bearer ${token}`, ...(body === undefined ? {} : { 'content-type': 'application/json' }) },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    const text = await res.text();
    let json: T | undefined;
    try {
      json = text ? (JSON.parse(text) as T) : undefined;
    } catch {
      json = undefined;
    }
    return { status: res.status, json, text: scrub(text, token).slice(0, 300) };
  }

  /** cloud.status: the last job and the virtual runners, never a token. */
  async status(): Promise<CloudStatus> {
    this.mustBeOn();
    return {
      ...(this.job ? { job: { ...this.job, steps: this.job.steps.map((s) => ({ ...s })) } } : {}),
      services: await this.services(),
      ...(this.lastCost ? { cost: { ...this.lastCost } } : {}),
    };
  }

  /**
   * cloud.cost: this month's cost of the boxes, from Fly's record of each
   * Machine's starts and stops at list price (fly-cost.ts). Read only: one
   * call to Fly, and only when asked (the section's refresh button).
   */
  async cost(): Promise<CloudCost> {
    this.mustBeOn();
    const token = this.envGet('FLY_API_TOKEN');
    const app = this.envGet('FLY_APP');
    if (!token || !app) throw new RpcError(ErrorCode.badParams, 'There is no cloud runner on Fly yet: set one up first.');
    rememberToken(token);
    const r = await this.fly<FlyMachine[]>(token, 'GET', `/apps/${encodeURIComponent(app)}/machines?include_deleted=true`);
    if (r.status >= 300 || !Array.isArray(r.json)) throw new RpcError(ErrorCode.other, `Fly answered ${r.status}: ${r.text}`);
    this.lastCost = estimate(app, r.json, this.deps.now());
    return { ...this.lastCost };
  }

  private async services(): Promise<Service[]> {
    try {
      const rows = (await this.deps.call('query', 'boxes:runnerServices', {})) as Service[];
      return (rows ?? []).map((r) => ({ serviceId: r.serviceId, name: r.name, provider: r.provider, image: r.image, box: r.box, running: r.running ?? 0, asleep: r.asleep ?? 0 }));
    } catch (e) {
      this.deps.log.warn('cloud: listing virtual runners failed', { error: String((e as Error)?.message ?? e) });
      return [];
    }
  }

  /** cloud.setup: starts in the background; cloud.status follows it. */
  setup(p: SetupParams): { started: true } {
    this.mustBeOn();
    const params = this.checkParams(p);
    return this.start('setup', SETUP, (steps) => this.runSetup(params, steps));
  }

  /** cloud.check: the same checks, nothing changed. */
  check(): { started: true } {
    this.mustBeOn();
    return this.start('check', CHECK, (steps) => this.runCheck(steps));
  }

  /** cloud.rebuild: the box image again, from this Mac's checkout. */
  rebuild(): { started: true } {
    this.mustBeOn();
    return this.start('rebuild', ['flyctl', 'token', 'image'], (steps) => this.runRebuild(steps));
  }

  /**
   * cloud.remove: the cloud runner out of Threadbear. Its threads must be
   * archived. Clears FLY_API_TOKEN, FLY_APP and FLY_ORG (never BOX_SEAL_KEY,
   * which other cloud tokens are sealed under), and with deleteApp deletes the
   * Fly app, its boxes and its image too; without it, says how to.
   */
  remove(p: RemoveParams = {}): { started: true } {
    this.mustBeOn();
    if (p.serviceId !== undefined && typeof p.serviceId !== 'string') throw bad('serviceId must be a string.');
    return this.start('remove', ['archived', 'unlist', 'unset', 'flyApp'], (steps) => this.runRemove(p, steps));
  }

  /** For tests and tb cloud: the job running now, if any. */
  wait(): Promise<void> {
    return this.going ?? Promise.resolve();
  }

  private checkParams(p: SetupParams): Required<Pick<SetupParams, 'org' | 'app' | 'name'>> & SetupParams {
    const token = p.token ? cleanToken(p.token) : undefined;
    if (token && !looksLikeFlyToken(token)) throw bad("That doesn't look like a Fly token (FlyV1 fm2_… from fly tokens create, or fo1_…).");
    if (token) rememberToken(token);
    const org = (p.org ?? 'personal').trim();
    if (!ORG_SLUG.test(org)) throw bad(`${JSON.stringify(org)} isn't a Fly organization name (lower case letters, digits and dashes).`);
    const app = (p.app ?? defaultAppName()).trim();
    if (!APP_NAME.test(app)) throw bad(`${JSON.stringify(app)} can't be a Fly app name: 3 to 63 lower case letters, digits and dashes, not starting or ending with a dash.`);
    const name = (p.name ?? 'Cloud (Fly)').trim();
    if (!name || name.length > 60) throw bad('Give the cloud runner a name of 1 to 60 characters.');
    const cpus = p.cpus ?? 1;
    const memoryMb = p.memoryMb ?? FLY_SUSPEND_MAX_MB;
    if (!Number.isSafeInteger(cpus) || cpus < 1 || cpus > 16) throw bad('CPUs must be a whole number from 1 to 16.');
    if (!Number.isSafeInteger(memoryMb) || memoryMb % MB_STEP || memoryMb < 1024 || memoryMb > FLY_SUSPEND_MAX_MB) throw bad(`Memory must be a multiple of 256 MB from 1024 to ${FLY_SUSPEND_MAX_MB} (Fly can't pause a bigger box).`);
    return { ...p, ...(token ? { token } : { token: undefined }), org, app, name, cpus, memoryMb };
  }

  private start(kind: 'setup' | 'check' | 'rebuild' | 'remove', ids: StepId[], body: (steps: Step[]) => Promise<void>): { started: true } {
    if (this.job?.running) throw new RpcError(ErrorCode.other, `A cloud ${this.job.kind} is already running. It shows in Settings, Runners.`);
    const steps: Step[] = ids.map((id) => ({ id, title: TITLES[id], state: 'todo' }));
    const job: NonNullable<CloudStatus['job']> = { kind, running: true, steps, startedAt: this.deps.now() };
    this.job = job;
    this.going = body(steps)
      .catch((e: unknown) => {
        const message = String((e as Error)?.message ?? e);
        job.error = message;
        for (const s of steps) if (s.state === 'doing') Object.assign(s, { state: 'fail', detail: s.detail ?? message });
        this.deps.log.warn(`cloud ${kind} stopped`, { error: message });
      })
      .finally(() => {
        for (const s of steps) if (s.state === 'todo') s.state = 'skipped';
        job.running = false;
        job.endedAt = this.deps.now();
      });
    return { started: true };
  }

  private step(steps: Step[], id: StepId): Step {
    const s = steps.find((x) => x.id === id);
    if (!s) throw new Error(`no step ${id}`);
    return s;
  }

  // Runs one step: a thrown error marks it failed and stops the rest.
  private async doStep(steps: Step[], id: StepId, body: (s: Step) => Promise<void>): Promise<void> {
    const s = this.step(steps, id);
    s.state = 'doing';
    delete s.detail;
    try {
      await body(s);
      if (s.state === 'doing') s.state = 'ok';
    } catch (e) {
      s.state = 'fail';
      s.detail = String((e as Error)?.message ?? e);
      throw new Error(`${s.title}: ${s.detail}`);
    }
  }

  private async flyctl(s: Step): Promise<void> {
    const r = await this.deps.exec('fly', ['version'], { timeoutMs: 15_000 });
    if (!r.ok) throw new Error('flyctl isn\'t installed on this Mac. Install it with: brew install flyctl');
    s.detail = lastLine(r.out).replace(/^fly(?:ctl)?\s+/, '');
  }

  // A token from flyctl's own sign-in: an org token named for Threadbear, a year long.
  private async mint(org: string): Promise<string> {
    const signedIn = await this.deps.exec('fly', ['auth', 'whoami'], { timeoutMs: 20_000 });
    if (!signedIn.ok) throw new Error('flyctl isn\'t signed in. Run fly auth login in a terminal, then try again, or paste a token.');
    const r = await this.deps.exec('fly', ['tokens', 'create', 'org', org, '--name', 'threadbear-boxes', '--expiry', '8760h'], { timeoutMs: 30_000 });
    const t = cleanToken(tokenLine(r.out));
    if (!r.ok || !looksLikeFlyToken(t)) throw new Error(`flyctl couldn't make a token for ${org}: ${scrub(lastLine(r.err) || 'no answer', t)}`);
    rememberToken(t);
    return t;
  }

  private async checkToken(token: string, org: string): Promise<void> {
    const r = await this.fly<{ total_apps?: number }>(token, 'GET', `/apps?org_slug=${encodeURIComponent(org)}`);
    if (r.status === 401 || r.status === 403) throw new Error(`Fly refused the token for ${org} (${r.status}). It needs to be an org token for that organization.`);
    if (r.status >= 300) throw new Error(`Fly answered ${r.status}: ${r.text}`);
  }

  private async ensureApp(token: string, org: string, app: string, s: Step, make: boolean): Promise<void> {
    const got = await this.fly(token, 'GET', `/apps/${encodeURIComponent(app)}`);
    if (got.status === 200) {
      s.detail = app;
      return;
    }
    if (got.status !== 404) throw new Error(`Fly answered ${got.status}: ${got.text}`);
    if (!make) throw new Error(`There's no app ${app} on Fly.`);
    const made = await this.fly(token, 'POST', '/apps', { app_name: app, org_slug: org });
    if (made.status === 422 || made.status === 409) throw new Error(`The name ${app} is taken on Fly. Pick another name.`);
    if (made.status >= 300) throw new Error(`Fly couldn't make ${app} (${made.status}): ${made.text}`);
    s.detail = `${app}, made`;
  }

  private envGet(name: string): string | undefined {
    const r = this.deps.convex(['env', 'get', name]);
    if (!r.ok) return undefined;
    const v = r.out.trim();
    return v || undefined;
  }

  private envSet(name: string, value: string): void {
    const r = this.deps.convex(['env', 'set', name], value);
    if (!r.ok) throw new Error(`Setting ${name} on your backend failed: ${scrub(r.err || 'no answer', value)}`);
  }

  private saveBackend(token: string, org: string, app: string, s: Step): void {
    // Only this Mac's deploy key can set them: a runner without one can't.
    const probe = this.deps.convex(['env', 'list']);
    if (!probe.ok) throw new Error(`This Mac can't change your backend's settings (${scrub(lastLine(probe.err) || 'no deploy key', token)}). Run the setup from the Mac that set Threadbear up.`);
    const done: string[] = [];
    if (this.envGet('FLY_API_TOKEN') !== token) (this.envSet('FLY_API_TOKEN', token), done.push('token'));
    if (this.envGet('FLY_APP') !== app) (this.envSet('FLY_APP', app), done.push('app'));
    if (this.envGet('FLY_ORG') !== org) (this.envSet('FLY_ORG', org), done.push('org'));
    // Never replaced: every sealed cloud token is under the one there.
    if (!this.envGet('BOX_SEAL_KEY')) (this.envSet('BOX_SEAL_KEY', crypto.randomBytes(32).toString('base64')), done.push('a new seal key'));
    s.detail = done.length ? `set ${done.join(', ')}` : 'already set';
  }

  // The image from this Mac's checkout on Fly's remote builder, the same build CI runs (box-image.yml).
  private async build(token: string, app: string, s: Step): Promise<void> {
    const runnerDir = path.join(this.deps.checkout(), 'runner');
    if (!fs.existsSync(path.join(runnerDir, 'box', 'Dockerfile'))) throw new Error(`There's no runner/box/Dockerfile in ${this.deps.checkout()}, so the image can't be built here.`);
    const commit = (await this.deps.exec('git', ['-C', runnerDir, 'rev-parse', 'HEAD'], { timeoutMs: 10_000 })).out.trim() || 'unknown';
    // flyctl wants a config: the app's name, and nothing else of it is deployed.
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tb-fly-'));
    const config = path.join(dir, 'fly.toml');
    fs.writeFileSync(config, `app = ${JSON.stringify(app)}\n`);
    s.detail = `building ${commit.slice(0, 9)} on Fly's builder`;
    try {
      const r = await this.deps.exec(
        'fly',
        ['deploy', '--build-only', '--push', '--remote-only', '--image-label', 'latest', '--config', config, '--dockerfile', path.join(runnerDir, 'box', 'Dockerfile'), '--build-arg', `COMMIT=${commit}`, '--build-arg', `BUILT_AT=${new Date(this.deps.now()).toISOString()}`, '-a', app, runnerDir],
        { env: { FLY_API_TOKEN: token }, cwd: runnerDir, timeoutMs: BUILD_MS },
      );
      if (!r.ok) throw new Error(`The build failed: ${scrub(lastLine(r.err) || lastLine(r.out) || 'no answer', token)}`);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
    s.detail = `registry.fly.io/${app}:latest from ${commit.slice(0, 9)}`;
  }

  // So the image follows main: what box-image.yml reads, on the checkout's GitHub repo. Optional.
  private async github(token: string, app: string, s: Step): Promise<void> {
    const cwd = this.deps.checkout();
    const auth = await this.deps.exec('gh', ['auth', 'status'], { cwd, timeoutMs: 15_000 });
    if (!auth.ok) {
      s.state = 'warn';
      s.detail = 'gh isn\'t signed in here, so the image updates only when you press Rebuild.';
      return;
    }
    // An org token can't make other tokens (Fly: "Not authorized to access
    // createlimitedaccesstoken"), so flyctl's own sign-in makes it when there
    // is one, and the org token is only the fallback.
    const args = ['tokens', 'create', 'deploy', '-a', app, '--name', 'threadbear-box-image', '--expiry', '8760h'];
    let deploy = await this.deps.exec('fly', args, { timeoutMs: 30_000 });
    if (!deploy.ok || !looksLikeFlyToken(cleanToken(tokenLine(deploy.out)))) deploy = await this.deps.exec('fly', args, { env: { FLY_API_TOKEN: token }, timeoutMs: 30_000 });
    const narrow = cleanToken(tokenLine(deploy.out));
    if (!deploy.ok || !looksLikeFlyToken(narrow)) {
      s.state = 'warn';
      s.detail = `flyctl couldn't make the image's own token, so the image updates only when you press Rebuild. Sign flyctl in (fly auth login), then press Edit… and Save. ${scrub(lastLine(deploy.err) || 'No answer', token)}`;
      return;
    }
    rememberToken(narrow);
    const v = await this.deps.exec('gh', ['variable', 'set', 'FLY_BOX_APP', '--body', app], { cwd, timeoutMs: 20_000 });
    const sec = await this.deps.exec('gh', ['secret', 'set', 'FLY_BOX_IMAGE_TOKEN'], { cwd, input: narrow, timeoutMs: 20_000 });
    if (!v.ok || !sec.ok) {
      s.state = 'warn';
      s.detail = `GitHub refused: ${scrub(lastLine(v.err) || lastLine(sec.err), narrow)}`;
      return;
    }
    s.detail = 'FLY_BOX_APP and FLY_BOX_IMAGE_TOKEN set; each push to main rebuilds it';
  }

  private async saveService(p: SetupParams & { name: string }, app: string, s: Step): Promise<void> {
    const existing = (await this.services()).find((x) => (p.serviceId ? x.serviceId === p.serviceId : x.provider === 'fly'));
    const id = (await this.deps.call('mutation', 'boxes:setRunnerService', {
      ...(existing ? { serviceId: existing.serviceId } : {}),
      name: p.name,
      provider: 'fly',
      image: `registry.fly.io/${app}:latest`,
      box: { cpus: p.cpus ?? existing?.box.cpus ?? 1, memoryMb: p.memoryMb ?? existing?.box.memoryMb ?? FLY_SUSPEND_MAX_MB },
    })) as string;
    s.detail = existing ? `${p.name}, updated` : `${p.name}, added`;
    this.deps.log.info('cloud runner saved', { serviceId: id });
  }

  // A box from the image with no boot token: its runner starts, finds none and exits 64. Then it's deleted.
  private async testBox(token: string, app: string, s: Step): Promise<void> {
    const sleep = this.deps.sleep ?? ((ms: number) => new Promise<void>((r) => setTimeout(r, ms)));
    const body = {
      name: `tb-test-${crypto.randomBytes(3).toString('hex')}`,
      config: { image: `registry.fly.io/${app}:latest`, guest: { cpu_kind: 'shared', cpus: 1, memory_mb: 1024 }, restart: { policy: 'no' }, auto_destroy: false, metadata: { tb_test: '1' } },
    };
    let made = await this.fly<{ id: string }>(token, 'POST', `/apps/${encodeURIComponent(app)}/machines`, body);
    // A just-pushed image can take a little while to reach the machines' side
    // of Fly's registry: it answers "manifest unknown" until then.
    for (let i = 0; i < 12 && made.status === 400 && /MANIFEST_UNKNOWN|manifest unknown/i.test(made.text); i++) {
      s.detail = 'waiting for Fly to see the new image';
      await sleep(5000);
      made = await this.fly<{ id: string }>(token, 'POST', `/apps/${encodeURIComponent(app)}/machines`, body);
    }
    if (made.status >= 300 || !made.json?.id) throw new Error(`Fly couldn't start a test box (${made.status}): ${made.text}`);
    const id = made.json.id;
    try {
      let exit: number | undefined;
      for (let i = 0; i < 40 && exit === undefined; i++) {
        await sleep(3000);
        const m = await this.fly<{ state: string; events?: { type: string; request?: { exit_event?: { exit_code?: number } } }[] }>(token, 'GET', `/apps/${encodeURIComponent(app)}/machines/${encodeURIComponent(id)}`);
        if (m.json?.state === 'stopped' || m.json?.state === 'failed') exit = m.json.events?.find((e) => e.type === 'exit')?.request?.exit_event?.exit_code ?? -1;
      }
      if (exit === undefined) throw new Error('The test box didn\'t finish in 2 minutes.');
      if (exit !== NO_BOOT_EXIT) throw new Error(`The test box's runner exited ${exit}, not ${NO_BOOT_EXIT} (no boot token). Its logs are in Fly's dashboard.`);
      s.detail = 'the image starts and its runner answers';
    } finally {
      const gone = await this.fly(token, 'DELETE', `/apps/${encodeURIComponent(app)}/machines/${encodeURIComponent(id)}?force=true`).catch(() => undefined);
      if (!gone || gone.status >= 300) this.deps.log.warn('cloud: the test box was left on Fly', { app, machine: id });
    }
  }

  private async runSetup(p: SetupParams & { org: string; app: string; name: string }, steps: Step[]): Promise<void> {
    let token = p.token;
    await this.doStep(steps, 'flyctl', (s) => this.flyctl(s));
    await this.doStep(steps, 'token', async (s) => {
      // A new paste wins; else the one already saved; else flyctl's sign-in makes one.
      token ??= this.envGet('FLY_API_TOKEN');
      if (token) rememberToken(token);
      if (!token && p.useFlyctl) token = await this.mint(p.org);
      if (!token) throw new Error('No Fly token: paste one, or use flyctl\'s sign-in.');
      await this.checkToken(token, p.org);
      s.detail = `org ${p.org}`;
    });
    const t = token as string;
    await this.doStep(steps, 'app', (s) => this.ensureApp(t, p.org, p.app, s, true));
    await this.doStep(steps, 'backend', async (s) => this.saveBackend(t, p.org, p.app, s));
    await this.doStep(steps, 'image', (s) => this.build(t, p.app, s));
    await this.doStep(steps, 'updates', (s) => this.github(t, p.app, s));
    await this.doStep(steps, 'service', (s) => this.saveService(p, p.app, s));
    await this.doStep(steps, 'test', (s) => this.testBox(t, p.app, s));
    this.deps.log.info('cloud runner set up', { app: p.app, org: p.org });
  }

  // What's on the backend now, read back: nothing is made or changed.
  private async saved(): Promise<{ token?: string; app?: string; org: string }> {
    const token = this.envGet('FLY_API_TOKEN');
    if (token) rememberToken(token);
    return { ...(token ? { token } : {}), ...(this.envGet('FLY_APP') ? { app: this.envGet('FLY_APP') } : {}), org: this.envGet('FLY_ORG') ?? 'personal' };
  }

  private async runCheck(steps: Step[]): Promise<void> {
    await this.doStep(steps, 'flyctl', (s) => this.flyctl(s)).catch(() => undefined);
    const now = await this.saved();
    await this.doStep(steps, 'token', async (s) => {
      if (!now.token) throw new Error('FLY_API_TOKEN isn\'t set on your backend.');
      await this.checkToken(now.token, now.org);
      s.detail = `org ${now.org}`;
    });
    await this.doStep(steps, 'app', async (s) => {
      if (!now.app) throw new Error('FLY_APP isn\'t set on your backend.');
      await this.ensureApp(now.token as string, now.org, now.app, s, false);
    });
    await this.doStep(steps, 'backend', async (s) => {
      if (!this.envGet('BOX_SEAL_KEY')) throw new Error('BOX_SEAL_KEY isn\'t set on your backend.');
      s.detail = 'FLY_API_TOKEN, FLY_APP and BOX_SEAL_KEY set';
    });
    await this.doStep(steps, 'service', async (s) => {
      const fly = (await this.services()).filter((x) => x.provider === 'fly');
      if (!fly.length) throw new Error('There\'s no Fly cloud runner yet.');
      s.detail = fly.map((x) => `${x.name}: ${x.running} running, ${x.asleep} asleep`).join('; ');
    });
  }

  private async runRemove(p: RemoveParams, steps: Step[]): Promise<void> {
    const now = await this.saved();
    const fly = (await this.services()).filter((x) => x.provider === 'fly');
    const svc = p.serviceId ? fly.find((x) => x.serviceId === p.serviceId) : fly[0];
    if (p.serviceId && !svc) throw new Error('There is no such cloud runner.');
    await this.doStep(steps, 'archived', async (s) => {
      if (!svc) {
        s.state = 'skipped';
        s.detail = 'no cloud runner listed';
        return;
      }
      if (svc.running + svc.asleep > 0) throw new Error(`${svc.name} has ${svc.running} running and ${svc.asleep} asleep. Archive those threads first, then remove it.`);
      s.detail = 'none running or asleep';
    });
    await this.doStep(steps, 'unlist', async (s) => {
      if (!svc) {
        s.state = 'skipped';
        return;
      }
      try {
        await this.deps.call('mutation', 'boxes:removeRunnerService', { serviceId: svc.serviceId });
      } catch (e) {
        const m = String((e as Error)?.message ?? e);
        if (/Could not find (public )?function|not found.*removeRunnerService|removeRunnerService.*not found/i.test(m)) throw new Error('Your backend is too old to remove a cloud runner. It updates with the runner; try again after that.');
        throw e;
      }
      s.detail = svc.name;
    });
    await this.doStep(steps, 'unset', async (s) => {
      const gone: string[] = [];
      for (const name of ['FLY_API_TOKEN', 'FLY_APP', 'FLY_ORG']) {
        if (!this.envGet(name)) continue;
        const r = this.deps.convex(['env', 'remove', name]);
        if (!r.ok) throw new Error(`Clearing ${name} on your backend failed: ${scrub(r.err || 'no answer', now.token ?? '')}`);
        gone.push(name);
      }
      s.detail = gone.length ? `cleared ${gone.join(', ')}; the seal key stays` : 'nothing was set';
    });
    await this.doStep(steps, 'flyApp', async (s) => {
      if (!now.app) {
        s.state = 'skipped';
        s.detail = 'no Fly app was set';
        return;
      }
      if (!p.deleteApp || !now.token) {
        s.detail = `${now.app} is still on Fly, with nothing running. Delete it with: fly apps destroy ${now.app}`;
        return;
      }
      const r = await this.fly(now.token, 'DELETE', `/apps/${encodeURIComponent(now.app)}`);
      if (r.status === 404) s.detail = `${now.app} was already gone`;
      else if (r.status >= 300) throw new Error(`Fly couldn't delete ${now.app} (${r.status}): ${r.text}. Delete it with: fly apps destroy ${now.app}`);
      else s.detail = `${now.app} deleted, with its boxes and image`;
    });
    this.lastCost = undefined;
  }

  private async runRebuild(steps: Step[]): Promise<void> {
    await this.doStep(steps, 'flyctl', (s) => this.flyctl(s));
    const now = await this.saved();
    await this.doStep(steps, 'token', async (s) => {
      if (!now.token || !now.app) throw new Error('Set up a cloud runner first: FLY_API_TOKEN and FLY_APP aren\'t set.');
      await this.checkToken(now.token, now.org);
      s.detail = `org ${now.org}`;
    });
    await this.doStep(steps, 'image', (s) => this.build(now.token as string, now.app as string, s));
  }
}

/** flyctl, gh and git, run with a time limit, their output kept in memory. The env given is added to this one's. */
export function realExec(cmd: string, args: string[], opts: { env?: NodeJS.ProcessEnv; input?: string; cwd?: string; timeoutMs?: number } = {}): Promise<Run> {
  return new Promise((resolve) => {
    const env: NodeJS.ProcessEnv = { ...process.env, ...opts.env };
    for (const k of Object.keys(env)) if (k.startsWith('THREADBEAR_') || k.startsWith('THREADBARE_')) delete env[k];
    let child: ReturnType<typeof spawn>;
    try {
      child = spawn(cmd, args, { cwd: opts.cwd, env, stdio: ['pipe', 'pipe', 'pipe'] });
    } catch (e) {
      resolve({ ok: false, out: '', err: String((e as Error).message) });
      return;
    }
    let out = '';
    let err = '';
    const cap = (s: string, more: string) => (s.length > 200_000 ? s : s + more);
    child.stdout?.on('data', (d: Buffer) => (out = cap(out, d.toString())));
    child.stderr?.on('data', (d: Buffer) => (err = cap(err, d.toString())));
    const timer = setTimeout(() => child.kill('SIGTERM'), opts.timeoutMs ?? 60_000);
    child.on('error', (e) => {
      clearTimeout(timer);
      resolve({ ok: false, out, err: err || String(e.message) });
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      resolve({ ok: code === 0, out, err, code });
    });
    if (opts.input !== undefined) child.stdin?.end(opts.input);
    else child.stdin?.end();
  });
}
