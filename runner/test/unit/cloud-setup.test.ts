// Setting up a Fly virtual runner from the Mac (cloud-setup.ts;
// docs/protocol/keys.md, "Cloud setup"): a fake Fly API, a fake flyctl and gh,
// and a fake backend's env. A second run changes nothing, BOX_SEAL_KEY is
// never replaced, and the token never shows in a step, the log or an error.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Logger } from '../../src/log.ts';
import { estimate, perSecond, startedSeconds, type FlyMachine } from '../../src/fly-cost.ts';
import { CloudSetup, cleanToken, githubRepo, defaultAppName, looksLikeFlyToken, type CloudDeps, type Run } from '../../src/cloud-setup.ts';

const TOKEN = `FlyV1 fm2_${'Ab1x'.repeat(12)}`;
// As long as real ones: flyctl's org and deploy tokens run to hundreds of characters.
const MINTED = `FlyV1 fm2_${'Zz9q'.repeat(120)},fm2_${'Zz9r'.repeat(40)}`;
const DEPLOY = `FlyV1 fm2_${'Dd4p'.repeat(120)},fm2_${'Dd4r'.repeat(40)}`;

type Calls = { exec: string[]; fly: string[]; convex: string[]; mutations: Record<string, unknown>[]; logs: string[] };

function checkout(): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tb-cloud-'));
  fs.mkdirSync(path.join(dir, 'runner', 'box'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'runner', 'box', 'Dockerfile'), 'FROM scratch\n');
  return dir;
}

function world(opts: { flyctl?: boolean; signedIn?: boolean; gh?: boolean; apps?: Set<string>; env?: Record<string, string>; exit?: number; badToken?: boolean; buildFails?: boolean; noDeployKey?: boolean; imageLateBy?: number; imageThere?: boolean; buildBlips?: number } = {}) {
  let late = opts.imageLateBy ?? 0;
  let blips = opts.buildBlips ?? 0;
  // What the registry has as :latest: a digest, which each build moves on.
  let digest = opts.imageThere ? `sha256:${'a'.repeat(64)}` : undefined;
  let builds = 0;
  const ghVars: Record<string, string> = {};
  const ghSecrets: string[] = [];
  const env: Record<string, string> = { ...opts.env };
  const apps = opts.apps ?? new Set<string>();
  const machines = new Map<string, { state: string }>();
  const services: { serviceId: string; name: string; provider: string; image: string; box: { cpus: number; memoryMb: number }; running: number; asleep: number }[] = [];
  const calls: Calls = { exec: [], fly: [], convex: [], mutations: [], logs: [] };
  const ok = (out = ''): Run => ({ ok: true, out, err: '' });
  const no = (err: string): Run => ({ ok: false, out: '', err });
  const log: Logger = {
    info: (m, x) => calls.logs.push(`${m} ${JSON.stringify(x ?? {})}`),
    warn: (m, x) => calls.logs.push(`${m} ${JSON.stringify(x ?? {})}`),
    error: (m, x) => calls.logs.push(`${m} ${JSON.stringify(x ?? {})}`),
    debug: () => {},
  };
  const deps: CloudDeps = {
    on: () => true,
    log,
    now: () => Date.parse('2026-11-23T12:00:00Z'),
    checkout: (() => {
      const dir = checkout();
      return () => dir;
    })(),
    sleep: async () => {},
    convex: (args, input) => {
      calls.convex.push(args.join(' '));
      if (opts.noDeployKey) return no('No CONVEX_DEPLOY_KEY');
      const [verb, name] = [args[1], args[2]];
      if (verb === 'list') return ok(Object.keys(env).map((k) => `${k}=…`).join('\n'));
      if (verb === 'get') return name && env[name] !== undefined ? ok(`${env[name]}\n`) : no(`Environment variable "${name}" not found`);
      if (verb === 'set' && name) {
        env[name] = input ?? '';
        return ok();
      }
      if (verb === 'remove' && name) {
        delete env[name];
        return ok();
      }
      return no('?');
    },
    exec: async (cmd, args, o) => {
      calls.exec.push(`${cmd} ${args.join(' ')}`);
      if (cmd === 'fly' && opts.flyctl === false) return no('spawn fly ENOENT');
      if (cmd === 'fly' && args[0] === 'version') return ok('fly v0.3.230 darwin/arm64 Commit: x\n');
      if (cmd === 'fly' && args[0] === 'auth') return opts.signedIn === false ? no('not logged in') : ok('me@example.com\n');
      if (cmd === 'fly' && args.slice(0, 3).join(' ') === 'tokens create org') return ok(`${MINTED}\n`);
      if (cmd === 'fly' && args.slice(0, 3).join(' ') === 'tokens create deploy') {
        // As Fly does: an org token can't make tokens; flyctl's own sign-in can.
        if (o?.env?.FLY_API_TOKEN) return no('Error: failed creating token: Not authorized to access createlimitedaccesstoken\n');
        return opts.signedIn === false ? no('Error: not logged in\n') : ok(`${DEPLOY}\n`);
      }
      if (cmd === 'fly' && args[0] === 'deploy') {
        assert.ok(o?.env?.FLY_API_TOKEN, 'the build gets the token in its environment');
        assert.ok(!args.join(' ').includes('fm2_'), 'never in its arguments');
        if (blips > 0) {
          blips--;
          return no('Error: failed to fetch an image or build from source: error building: failed to receive status: rpc error: code = Unavailable desc = error reading from server: read tcp 192.168.1.178:58699->149.248.212.172:443: read: operation timed out\n');
        }
        if (opts.buildFails) return no(`Error: unauthorized ${TOKEN}\n`);
        builds++;
        digest = `sha256:${String(builds).repeat(64)}`;
        return ok('image: registry.fly.io/x:latest\n');
      }
      if (cmd === 'git' && args.includes('remote')) return ok('https://github.com/someone/threadbear.git\n');
      if (cmd === 'git') return ok('0123456789abcdef\n');
      if (cmd === 'gh' && args[0] === 'auth') return opts.gh === false ? no('not logged in') : ok();
      if (cmd === 'gh') {
        // Always the checkout's own repo, never gh's guess between remotes.
        assert.equal(args[args.indexOf('-R') + 1], 'someone/threadbear');
        if (args[0] === 'variable' && args[1] === 'get') return ghVars.FLY_BOX_APP ? ok(`${ghVars.FLY_BOX_APP}\n`) : no('variable FLY_BOX_APP was not found');
        if (args[0] === 'secret' && args[1] === 'list') return ok(ghSecrets.map((n) => `${n}\t2026-11-24`).join('\n'));
        if (args[0] === 'variable' && args[1] === 'set') ghVars[args[2]!] = args[args.indexOf('--body') + 1]!;
        if (args[0] === 'secret' && args[1] === 'set') {
          assert.equal(o?.input, DEPLOY);
          ghSecrets.push(args[2]!);
        }
        return ok();
      }
      return no(`unexpected ${cmd}`);
    },
    fetch: async (url, init) => {
      const u = new URL(url);
      const method = init?.method ?? 'GET';
      calls.fly.push(`${method} ${u.pathname}${u.search}`);
      const auth = (init?.headers as Record<string, string>)?.authorization;
      const json = (status: number, body: unknown) => new Response(JSON.stringify(body), { status });
      if (u.host === 'registry.fly.io') {
        // As Fly's registry does: org tokens are refused; flyctl's own sign-in is let in.
        const who = Buffer.from(auth?.replace(/^Basic /, '') ?? '', 'base64').toString();
        if (who !== 'x:me@example.com') return new Response(null, { status: 401 });
        return digest ? new Response(null, { status: 200, headers: { 'docker-content-digest': digest } }) : new Response(null, { status: 404 });
      }
      if (opts.badToken || ![TOKEN, MINTED].map((t) => `Bearer ${t}`).includes(auth ?? '')) return json(401, { error: 'unauthorized' });
      const m = u.pathname.match(/^\/v1\/apps(?:\/([^/]+))?(?:\/machines(?:\/([^/]+))?)?$/);
      if (!m) return json(404, {});
      const [, app, machine] = m;
      if (!app && method === 'GET') return json(200, { total_apps: apps.size, apps: [] });
      if (!app && method === 'POST') {
        const body = JSON.parse(String(init?.body)) as { app_name: string };
        if (apps.has(body.app_name)) return json(422, { error: 'taken' });
        apps.add(body.app_name);
        return json(201, {});
      }
      if (method === 'DELETE' && app && !u.pathname.includes('/machines')) return apps.delete(app) ? json(202, {}) : json(404, { error: 'not found' });
      if (app && !u.pathname.includes('/machines')) return apps.has(app) ? json(200, { name: app }) : json(404, { error: 'not found' });
      if (method === 'POST' && !machine && late > 0) {
        late--;
        return json(400, { error: 'failed to get manifest registry.fly.io/x:latest: {"errors":[{"code":"MANIFEST_UNKNOWN"}]}' });
      }
      if (method === 'POST' && !machine) {
        const id = `m${machines.size + 1}`;
        machines.set(id, { state: 'started' });
        return json(200, { id });
      }
      if (method === 'GET' && !machine) return json(200, []);
      if (method === 'GET' && machine) return json(200, { state: 'stopped', events: [{ type: 'exit', request: { exit_event: { exit_code: opts.exit ?? 64 } } }] });
      if (method === 'DELETE' && machine) {
        machines.delete(machine);
        return json(200, { ok: true });
      }
      return json(404, {});
    },
    call: async (kind, fn, args) => {
      if (fn === 'boxes:runnerServices') return services;
      if (fn === 'boxes:removeRunnerService') {
        calls.mutations.push({ removed: args.serviceId });
        const i = services.findIndex((s) => s.serviceId === args.serviceId);
        if (i >= 0) services.splice(i, 1);
        return null;
      }
      if (fn === 'boxes:setRunnerService') {
        calls.mutations.push(args);
        const id = (args.serviceId as string) ?? `s${services.length + 1}`;
        const row = { serviceId: id, name: String(args.name), provider: 'fly', image: String(args.image), box: args.box as { cpus: number; memoryMb: number }, running: 0, asleep: 0 };
        const i = services.findIndex((s) => s.serviceId === id);
        if (i >= 0) services[i] = row;
        else services.push(row);
        return id;
      }
      throw new Error(`unexpected ${kind} ${fn}`);
    },
  };
  return { deps, env, apps, machines, calls, services };
}

function noToken(w: { calls: Calls }, setup: CloudSetup, extra: string[] = []): Promise<void> {
  return setup.status().then((st) => {
    const all = JSON.stringify(st) + w.calls.logs.join('\n') + w.calls.exec.join('\n') + extra.join('\n');
    for (const t of [TOKEN, MINTED, DEPLOY]) assert.ok(!all.includes(t.slice(10)), 'no token anywhere it could be seen');
  });
}

test('a pasted token sets up everything once, and a second run changes nothing', async () => {
  const w = world();
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: `  ${TOKEN}\n`, app: 'tb-boxes-wayne' });
  await setup.wait();
  const st = await setup.status();
  assert.equal(st.job?.error, undefined, st.job?.error ?? "");
  assert.deepEqual(st.job?.steps.map((s) => `${s.id}:${s.state}`), ['flyctl:ok', 'token:ok', 'app:ok', 'backend:ok', 'image:ok', 'updates:ok', 'service:ok', 'test:ok']);
  assert.ok(w.apps.has('tb-boxes-wayne'));
  assert.equal(w.env.FLY_API_TOKEN, TOKEN);
  assert.equal(w.env.FLY_APP, 'tb-boxes-wayne');
  assert.equal(w.env.FLY_ORG, 'personal');
  assert.equal(Buffer.from(w.env.BOX_SEAL_KEY ?? '', 'base64').length, 32);
  // Pinned to the digest it pushed: a box never boots a tag (convex/boxImage.ts).
  assert.deepEqual(w.calls.mutations[0], { name: 'Cloud (Fly)', provider: 'fly', image: `registry.fly.io/tb-boxes-wayne@sha256:${'1'.repeat(64)}`, box: { cpus: 1, memoryMb: 2048 } });
  assert.equal(w.machines.size, 0, 'the test box is deleted');
  assert.deepEqual(st.services.map((s) => s.name), ['Cloud (Fly)']);
  await noToken(w, setup);

  const seal = w.env.BOX_SEAL_KEY;
  const sets = w.calls.convex.filter((c) => c.startsWith('env set')).length;
  setup.setup({ app: 'tb-boxes-wayne' });
  await setup.wait();
  const again = await setup.status();
  assert.equal(again.job?.error, undefined, again.job?.error ?? "");
  assert.equal(w.calls.convex.filter((c) => c.startsWith('env set')).length, sets, 'nothing set the second time');
  assert.equal(w.env.BOX_SEAL_KEY, seal);
  assert.equal(again.job?.steps.find((s) => s.id === 'backend')?.detail, 'already set');
  assert.equal(w.calls.mutations.length, 2);
  assert.equal(w.calls.mutations[1]?.serviceId, 's1', 'the same runner, updated');
});

test('an existing seal key is never replaced', async () => {
  const w = world({ env: { BOX_SEAL_KEY: 'keep-me' } });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-x' });
  await setup.wait();
  assert.equal(w.env.BOX_SEAL_KEY, 'keep-me');
});

test("flyctl's sign-in makes an org token when none is saved", async () => {
  const w = world();
  const setup = new CloudSetup(w.deps);
  setup.setup({ useFlyctl: true, app: 'tb-boxes-y', org: 'acme' });
  await setup.wait();
  assert.equal((await setup.status()).job?.error, undefined);
  assert.equal(w.env.FLY_API_TOKEN, MINTED);
  assert.ok(w.calls.exec.includes('fly tokens create org acme --name threadbear-boxes --expiry 8760h'));
  assert.equal(w.env.FLY_ORG, 'acme');
  await noToken(w, setup);
});

test("flyctl missing stops at the first step with how to install it", async () => {
  const w = world({ flyctl: false });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN });
  await setup.wait();
  const st = await setup.status();
  assert.equal(st.job?.steps[0]?.state, 'fail');
  assert.match(st.job?.steps[0]?.detail ?? '', /brew install flyctl/);
  assert.ok(st.job?.steps.slice(1).every((s) => s.state === 'skipped'));
  assert.equal(Object.keys(w.env).length, 0);
});

test('a refused token stops before anything is made, and its error has no token', async () => {
  const w = world({ badToken: true });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN });
  await setup.wait();
  const st = await setup.status();
  assert.equal(st.job?.steps.find((s) => s.id === 'token')?.state, 'fail');
  assert.match(st.job?.error ?? '', /refused the token/);
  assert.equal(w.apps.size, 0);
  await noToken(w, setup, [st.job?.error ?? '']);
});

test('a build that fails says so, with the token scrubbed from flyctl\'s words', async () => {
  const w = world({ buildFails: true });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-z' });
  await setup.wait();
  const st = await setup.status();
  const image = st.job?.steps.find((s) => s.id === 'image');
  assert.equal(image?.state, 'fail');
  assert.match(image?.detail ?? '', /REDACTED/);
  assert.equal(w.calls.mutations.length, 0, 'no runner without an image');
  await noToken(w, setup, [st.job?.error ?? '']);
});

test("without gh signed in the image step still passes, and only Rebuild updates it", async () => {
  const w = world({ gh: false });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-g' });
  await setup.wait();
  const st = await setup.status();
  assert.equal(st.job?.error, undefined);
  assert.equal(st.job?.steps.find((s) => s.id === 'updates')?.state, 'warn');
});

test('a test box waits for Fly to see a just-pushed image', async () => {
  const w = world({ imageLateBy: 3 });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-l' });
  await setup.wait();
  const st = await setup.status();
  assert.equal(st.job?.error, undefined);
  assert.equal(st.job?.steps.find((s) => s.id === 'test')?.state, 'ok');
  assert.equal(w.machines.size, 0);
});

test("the image's own token comes from flyctl's sign-in, not the org token", async () => {
  const w = world();
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-d' });
  await setup.wait();
  assert.equal((await setup.status()).job?.steps.find((s) => s.id === 'updates')?.state, 'ok');
  const out = world({ signedIn: false });
  const other = new CloudSetup(out.deps);
  other.setup({ token: TOKEN, app: 'tb-boxes-e' });
  await other.wait();
  const st = await other.status();
  // Signed out, the registry won't name the image's digest, and a box only
  // boots a pinned one (convex/boxImage.ts): it says to sign in.
  assert.match(st.job?.error ?? '', /fly auth login/);
  assert.equal(out.services.length, 0, 'no runner row on a tag');
});

test('a test box that exits otherwise fails the setup and is still deleted', async () => {
  const w = world({ exit: 1 });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-t' });
  await setup.wait();
  const st = await setup.status();
  assert.equal(st.job?.steps.find((s) => s.id === 'test')?.state, 'fail');
  assert.equal(w.machines.size, 0);
});

test('a Mac without the deploy key says where to run it, before changing anything', async () => {
  const w = world({ noDeployKey: true });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-k' });
  await setup.wait();
  const st = await setup.status();
  assert.match(st.job?.steps.find((s) => s.id === 'backend')?.detail ?? '', /from the Mac that set Threadbear up/);
});

test('check reads what is there and changes nothing', async () => {
  const w = world({ env: { FLY_API_TOKEN: TOKEN, FLY_APP: 'tb-boxes-c', BOX_SEAL_KEY: 'k' }, apps: new Set(['tb-boxes-c']) });
  const setup = new CloudSetup(w.deps);
  setup.check();
  await setup.wait();
  const st = await setup.status();
  assert.deepEqual(st.job?.steps.map((s) => `${s.id}:${s.state}`), ['flyctl:ok', 'token:ok', 'app:ok', 'backend:ok', 'service:fail']);
  assert.ok(!w.calls.convex.some((c) => c.startsWith('env set')));
  assert.ok(!w.calls.fly.some((c) => c.startsWith('POST')));
});

test('one job at a time, bad names refused, and nothing without the experiment', async () => {
  const w = world();
  const setup = new CloudSetup(w.deps);
  assert.throws(() => setup.setup({ token: 'nope' }), /doesn't look like a Fly token/);
  assert.throws(() => setup.setup({ token: TOKEN, app: 'Bad_Name' }), /can't be a Fly app name/);
  assert.throws(() => setup.setup({ token: TOKEN, memoryMb: 4096 }), /Fly can't pause a bigger box/);
  setup.setup({ token: TOKEN, app: 'tb-boxes-a' });
  assert.throws(() => setup.check(), /already running/);
  await setup.wait();
  const off = new CloudSetup({ ...w.deps, on: () => false });
  await assert.rejects(off.status(), /aren't on/);
});

test('token and app name helpers', () => {
  assert.equal(cleanToken(' FlyV1   fm2_abc\n'), 'FlyV1 fm2_abc');
  assert.ok(looksLikeFlyToken(TOKEN));
  assert.ok(!looksLikeFlyToken('sk-ant-oat01-x'));
  assert.equal(defaultAppName('Wayne.Sutton'), 'tb-boxes-wayne-sutton');
  assert.equal(defaultAppName('__'), 'tb-boxes-me');
});

test("this month's cost: started time from Fly's events, priced by size, deleted boxes too", async () => {
  const now = Date.parse('2026-11-24T12:00:00Z');
  const hour = 3600_000;
  const small = { cpu_kind: 'shared', cpus: 1, memory_mb: 1024 };
  const big = { cpu_kind: 'shared', cpus: 2, memory_mb: 2048 };
  const machines: FlyMachine[] = [
    // Two hours, then deleted.
    { id: 'a', state: 'destroyed', config: { guest: small }, events: [
      { status: 'created', timestamp: now - 10 * hour }, { status: 'started', timestamp: now - 9 * hour },
      { status: 'stopped', timestamp: now - 7 * hour }, { status: 'destroyed', timestamp: now - 6 * hour },
    ] },
    // Started an hour ago and still going.
    { id: 'b', state: 'started', config: { guest: big }, events: [{ status: 'started', timestamp: now - hour }] },
    // Last month only: nothing this month.
    { id: 'c', state: 'stopped', config: { guest: small }, events: [
      { status: 'started', timestamp: Date.parse('2026-10-30T00:00:00Z') }, { status: 'suspended', timestamp: Date.parse('2026-10-30T05:00:00Z') },
    ] },
  ];
  const c = estimate('tb-boxes-x', machines, now);
  assert.equal(c.boxes, 2);
  assert.equal(c.hours, 3);
  // 2 h of 1 CPU 1 GB ($6.69 a month) and 1 h of 2 CPUs 2 GB ($13.38 a month).
  const want = (2 * 3600 * 6.69 + 3600 * 13.38) / (30 * 24 * 3600);
  assert.equal(c.usd, Math.round(want * 100) / 100);
  assert.equal(c.monthStart, Date.parse('2026-11-01T00:00:00Z'));
  // A box across the month's start counts only from the 1st.
  assert.equal(startedSeconds({ id: 'd', state: 'stopped', events: [{ status: 'started', timestamp: Date.parse('2026-10-31T23:00:00Z') }, { status: 'stopped', timestamp: Date.parse('2026-11-01T01:00:00Z') }] }, c.monthStart, now), 3600);
  assert.ok(Math.abs(perSecond(small) * 30 * 24 * 3600 - 6.69) < 0.001);
});

test('cloud.cost reads the app and token from the backend, asks Fly once, and status keeps the answer', async () => {
  const w = world({ env: { FLY_API_TOKEN: TOKEN, FLY_APP: 'tb-boxes-c' }, apps: new Set(['tb-boxes-c']) });
  const setup = new CloudSetup(w.deps);
  const c = await setup.cost();
  assert.equal(c.app, 'tb-boxes-c');
  assert.equal(w.calls.fly.filter((x) => x.startsWith('GET /v1/apps/tb-boxes-c/machines')).length, 1);
  assert.ok(w.calls.fly.some((x) => x.includes('include_deleted=true')));
  assert.deepEqual((await setup.status()).cost, c);
  const none = new CloudSetup(world().deps);
  await assert.rejects(none.cost(), /set one up first/);
});

test('remove: the runner row and the Fly settings go, the seal key stays, and the app only when asked', async () => {
  const w = world();
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-r' });
  await setup.wait();
  const seal = w.env.BOX_SEAL_KEY;
  setup.remove();
  await setup.wait();
  let st = await setup.status();
  assert.equal(st.job?.kind, 'remove');
  assert.equal(st.job?.error, undefined);
  assert.equal(st.services.length, 0);
  assert.deepEqual([w.env.FLY_API_TOKEN, w.env.FLY_APP, w.env.FLY_ORG], [undefined, undefined, undefined]);
  assert.equal(w.env.BOX_SEAL_KEY, seal);
  assert.ok(w.apps.has('tb-boxes-r'), 'the Fly app stays without deleteApp');
  assert.match(st.job?.steps.find((s) => s.id === 'flyApp')?.detail ?? '', /fly apps destroy tb-boxes-r/);
  await noToken(w, setup);

  const d = world();
  const two = new CloudSetup(d.deps);
  two.setup({ token: TOKEN, app: 'tb-boxes-d2' });
  await two.wait();
  two.remove({ deleteApp: true });
  await two.wait();
  st = await two.status();
  assert.equal(st.job?.error, undefined);
  assert.ok(!d.apps.has('tb-boxes-d2'));
  assert.ok(d.calls.fly.includes('DELETE /v1/apps/tb-boxes-d2'));
});

test('remove waits for the threads to be archived, and changes nothing until then', async () => {
  const w = world();
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-busy' });
  await setup.wait();
  w.services[0].asleep = 2;
  setup.remove({ deleteApp: true });
  await setup.wait();
  const st = await setup.status();
  assert.match(st.job?.error ?? '', /Archive those threads first/);
  assert.equal(st.services.length, 1);
  assert.equal(w.env.FLY_APP, 'tb-boxes-busy');
  assert.ok(w.apps.has('tb-boxes-busy'));
});

test("the image token goes to the checkout's own GitHub repo once, and a second Save makes no new token", async () => {
  const w = world();
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-g2' });
  await setup.wait();
  assert.match((await setup.status()).job?.steps.find((s) => s.id === 'updates')?.detail ?? '', /someone\/threadbear/);
  const made = w.calls.exec.filter((x) => x.startsWith('fly tokens create deploy')).length;
  setup.setup({ app: 'tb-boxes-g2' });
  await setup.wait();
  assert.match((await setup.status()).job?.steps.find((s) => s.id === 'updates')?.detail ?? '', /already set/);
  assert.equal(w.calls.exec.filter((x) => x.startsWith('fly tokens create deploy')).length, made);
  assert.equal(githubRepo('git@github.com:a/b.git'), 'a/b');
  assert.equal(githubRepo('https://github.com/waynesutton/threadbear.git'), 'waynesutton/threadbear');
  assert.equal(githubRepo('https://gitlab.com/a/b.git'), undefined);
});

test('setting up again skips the build when the image is on Fly already', async () => {
  const w = world({ imageThere: true });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-again' });
  await setup.wait();
  const st = await setup.status();
  const image = st.job?.steps.find((s) => s.id === 'image');
  assert.equal(image?.state, 'ok');
  assert.match(image?.detail ?? '', /there already/);
  assert.ok(!w.calls.exec.some((c) => c.startsWith('fly deploy')), 'no build');
  assert.ok(w.calls.fly.includes('HEAD /v2/tb-boxes-again/manifests/latest'));
  await noToken(w, setup, [st.job?.error ?? '']);
});

test("the build tries again when Fly's builder drops the connection", async () => {
  const w = world({ buildBlips: 2 });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-blip' });
  await setup.wait();
  const st = await setup.status();
  assert.equal(st.job?.steps.find((s) => s.id === 'image')?.state, 'ok');
  assert.equal(w.calls.exec.filter((c) => c.startsWith('fly deploy')).length, 3);
});

test("a builder that keeps dropping says it's Fly's side, not the build", async () => {
  const w = world({ buildBlips: 5 });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-down' });
  await setup.wait();
  const image = (await setup.status()).job?.steps.find((s) => s.id === 'image');
  assert.equal(image?.state, 'fail');
  assert.match(image?.detail ?? '', /try again in a few minutes/);
  assert.equal(w.calls.exec.filter((c) => c.startsWith('fly deploy')).length, 3);
});

test('Rebuild pins the cloud runner to the new image, and setting up again keeps the one there', async () => {
  const w = world({ imageThere: true });
  const setup = new CloudSetup(w.deps);
  setup.setup({ token: TOKEN, app: 'tb-boxes-pin' });
  await setup.wait();
  assert.equal((await setup.status()).job?.error, undefined);
  assert.equal(w.services[0]?.image, `registry.fly.io/tb-boxes-pin@sha256:${'a'.repeat(64)}`, 'the digest already there');
  setup.rebuild();
  await setup.wait();
  const st = await setup.status();
  assert.equal(st.job?.error, undefined, st.job?.error ?? '');
  assert.deepEqual(st.job?.steps.map((s) => `${s.id}:${s.state}`), ['flyctl:ok', 'token:ok', 'image:ok', 'service:ok']);
  assert.equal(w.services.length, 1);
  assert.equal(w.services[0]?.image, `registry.fly.io/tb-boxes-pin@sha256:${'1'.repeat(64)}`, 'the digest the build pushed');
  await noToken(w, setup);
});
