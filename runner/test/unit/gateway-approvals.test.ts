// Gateway approvals on a card in the thread (gateway-approvals.ts,
// prds/gateway-approvals-in-threadbear.md): a call the Connectors gateway
// holds is matched to the thread that made it, shown as an ordinary
// approval card, and Allow or Deny goes back to the gateway.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { makeHarness, tick, tmpDir } from '../helpers/fakes.ts';
import { FakeClock } from '../../src/clock.ts';
import {
  approvalIdsIn,
  cardTitle,
  GatewayApprovals,
  GatewayApprovalsClient,
  gatewayTool,
  NO_API_BACKOFF_MS,
  POLL_MS,
  sameArgs,
  WATCH_MS,
  type GatewayApproval,
  type GatewayCardHost,
} from '../../src/gateway-approvals.ts';

const BASE = 'http://127.0.0.1:8795';
const KEY = 'ab'.repeat(32);

test('only a service tool of the gateway is watched, never its own', () => {
  assert.equal(gatewayTool('mcp__connectors__slack__slack_send_message'), 'slack__slack_send_message');
  assert.equal(gatewayTool('mcp__connectors__connectors__await_approval'), undefined);
  assert.equal(gatewayTool('mcp__connectors__slack'), undefined);
  assert.equal(gatewayTool('mcp__threadbear__end_turn'), undefined);
  assert.equal(gatewayTool('Bash'), undefined);
});

test('arguments match whole, or by the start of a string the event cut', () => {
  assert.ok(sameArgs({ channel_id: 'C1', message: 'hi' }, { message: 'hi', channel_id: 'C1' }));
  assert.ok(!sameArgs({ channel_id: 'C1', message: 'hi' }, { channel_id: 'C2', message: 'hi' }));
  assert.ok(!sameArgs({ channel_id: 'C1' }, { channel_id: 'C1', message: 'hi' }));
  assert.ok(sameArgs({ a: [1, { b: 'x' }] }, { a: [1, { b: 'x' }] }));
  assert.ok(!sameArgs({ a: [1, 2] }, { a: [1] }));
  const long = 'x'.repeat(40_000);
  assert.ok(sameArgs({ message: long.slice(0, 16 * 1024) }, { message: long }));
  assert.ok(!sameArgs({ message: 'short' }, { message: 'short and more' }));
});

test('approval ids are read only from links on the gateway itself', () => {
  const text = `Waiting for Wayne to approve this at ${BASE}/approvals/ap_NwbtlDV_akcFJI9p. And https://evil.example/approvals/ap_EvilEvilEvil1.`;
  assert.deepEqual(approvalIdsIn(text, BASE), ['ap_NwbtlDV_akcFJI9p']);
  assert.deepEqual(approvalIdsIn(text, 'not a url'), []);
});

test('the card says which service and what it does', () => {
  assert.equal(cardTitle({ upstream: 'slack', upstreamTool: 'slack_send_message' }, 'Slack'), 'Slack: send message');
  assert.equal(cardTitle({ upstream: 'gmail', upstreamTool: 'send_message' }), 'Gmail: send message');
  assert.equal(cardTitle({ upstream: 'linear', upstreamTool: 'save_issue' }, 'Linear'), 'Linear: save issue');
});

// A gateway with /api/approvals, in memory, behind the real client.
function fakeGateway(opts: { noApi?: boolean } = {}) {
  const dir = tmpDir();
  const keyFile = path.join(dir, 'approvals-key');
  fs.writeFileSync(keyFile, `${KEY}\n`, { mode: 0o600 });
  const approvals = new Map<string, GatewayApproval>();
  const requests: { method: string; path: string; auth: string | null }[] = [];
  const fetchImpl = (async (input: URL | string, init: RequestInit = {}) => {
    const url = new URL(String(input));
    const auth = new Headers(init.headers).get('authorization');
    requests.push({ method: init.method ?? 'GET', path: url.pathname, auth });
    const json = (status: number, body: unknown) => new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
    if (opts.noApi) return json(404, { error: 'not found' });
    if (auth !== `Bearer ${KEY}`) return json(401, { error: 'unauthorized' });
    if (url.pathname === '/api/approvals') return json(200, { approvals: [...approvals.values()].filter((a) => a.status === 'pending') });
    const id = decodeURIComponent(url.pathname.split('/').pop()!);
    const a = approvals.get(id);
    if (!a) return json(404, { error: 'not_found' });
    if (init.method === 'POST') {
      if (a.status !== 'pending') return json(409, { error: 'already_decided', approval: a });
      const { decision } = JSON.parse(String(init.body)) as { decision: string };
      a.status = decision === 'approve' ? 'approved' : 'declined';
    }
    return json(200, a);
  }) as typeof fetch;
  const client = new GatewayApprovalsClient(BASE, keyFile, fetchImpl);
  const hold = (id: string, tool: string, args: Record<string, unknown>) => {
    const [upstream, upstreamTool] = tool.split('__') as [string, string];
    approvals.set(id, { id, status: 'pending', agent: 'claude-3', upstream, tool, upstreamTool, args, url: `${BASE}/approvals/${id}` });
  };
  return { client, approvals, requests, hold, keyFile };
}

function fakeHost() {
  const shown: { threadId: string; cardId: string; title: string; approval: GatewayApproval; decide: (d: 'approve' | 'decline') => Promise<GatewayApproval | undefined> }[] = [];
  const closed: { threadId: string; cardId: string; status?: string }[] = [];
  const host: GatewayCardHost = {
    showGatewayCard: (threadId, card) => {
      shown.push({ threadId, ...card });
      return true;
    },
    closeGatewayCard: (threadId, cardId, a) => {
      closed.push({ threadId, cardId, status: a?.status });
    },
  };
  return { host, shown, closed };
}

const SEND = { type: 'tool.call', itemId: 'i1', name: 'mcp__connectors__slack__slack_send_message', input: { channel_id: 'U1', message: 'hello' } };

test('a held call becomes a card in the thread whose call it is, while it is still held', async () => {
  const clock = new FakeClock();
  const g = fakeGateway();
  const h = fakeHost();
  const w = new GatewayApprovals({ client: () => g.client, host: h.host, clock, serviceTitle: () => 'Slack' });
  // Two threads call the same tool; only the arguments tell them apart.
  w.observe('t-other', { ...SEND, input: { channel_id: 'C9', message: 'other' } });
  w.observe('t-mine', SEND);
  g.hold('ap_mine0000001', 'slack__slack_send_message', { channel_id: 'U1', message: 'hello' });
  await w.poll();
  assert.equal(h.shown.length, 1);
  assert.equal(h.shown[0].threadId, 't-mine');
  assert.equal(h.shown[0].cardId, 'gw-ap_mine0000001');
  assert.equal(h.shown[0].title, 'Slack: send message');
  // Every request carried the key, and only to the gateway's API.
  assert.ok(g.requests.every((r) => r.auth === `Bearer ${KEY}` && r.path.startsWith('/api/approvals')));
  // Seen once: a later poll doesn't card it again.
  await w.poll();
  assert.equal(h.shown.length, 1);
  w.stop();
});

test('a call nobody is watching, or one past its watch, gets no card', async () => {
  const clock = new FakeClock();
  const g = fakeGateway();
  const h = fakeHost();
  const w = new GatewayApprovals({ client: () => g.client, host: h.host, clock });
  g.hold('ap_stranger0001', 'slack__slack_send_message', { channel_id: 'U1', message: 'hello' });
  await w.poll();
  w.observe('t1', SEND);
  clock.advance(WATCH_MS + 1);
  await w.poll();
  assert.equal(h.shown.length, 0);
  w.stop();
});

test('the approval id in a tool result cards it too, after the hold', async () => {
  const clock = new FakeClock();
  const g = fakeGateway();
  const h = fakeHost();
  const w = new GatewayApprovals({ client: () => g.client, host: h.host, clock });
  g.hold('ap_late00000001', 'gmail__send_message', { to: 'sam@example.com' });
  w.observe('t1', { type: 'tool.result', itemId: 'i1', text: `Waiting for Wayne to approve this at ${BASE}/approvals/ap_late00000001.` });
  await tick();
  await tick();
  assert.equal(h.shown.length, 1);
  assert.equal(h.shown[0].threadId, 't1');
  w.stop();
});

test('a card decided on the gateway page, or expired there, closes', async () => {
  const clock = new FakeClock();
  const g = fakeGateway();
  const h = fakeHost();
  const w = new GatewayApprovals({ client: () => g.client, host: h.host, clock });
  w.observe('t1', SEND);
  g.hold('ap_page00000001', 'slack__slack_send_message', { channel_id: 'U1', message: 'hello' });
  await w.poll();
  g.approvals.get('ap_page00000001')!.status = 'approved';
  await w.poll();
  assert.deepEqual(h.closed, [{ threadId: 't1', cardId: 'gw-ap_page00000001', status: 'approved' }]);
  w.stop();
});

test('no key or a gateway off this Mac: nothing is watched or asked', async () => {
  const clock = new FakeClock();
  const g = fakeGateway();
  fs.rmSync(g.keyFile);
  const h = fakeHost();
  const w = new GatewayApprovals({ client: () => g.client, host: h.host, clock });
  w.observe('t1', SEND);
  await w.poll();
  assert.equal(g.requests.length, 0);
  const remote = new GatewayApprovalsClient('https://mac.example.ts.net:8443', g.keyFile);
  assert.equal(remote.usable(), false);
  w.stop();
});

test('a gateway without the API is left alone for a while', async () => {
  const clock = new FakeClock();
  const g = fakeGateway({ noApi: true });
  const h = fakeHost();
  const w = new GatewayApprovals({ client: () => g.client, host: h.host, clock });
  w.observe('t1', SEND);
  await w.poll();
  assert.equal(g.requests.length, 1);
  w.observe('t1', SEND);
  await w.poll();
  assert.equal(g.requests.length, 1);
  clock.advance(NO_API_BACKOFF_MS + 1);
  w.observe('t1', SEND);
  await w.poll();
  assert.equal(g.requests.length, 2);
  w.stop();
});

// The whole way through a runner: the agent's call, the card, Allow.
function runnerWithGateway() {
  const h = makeHarness(tmpDir());
  const g = fakeGateway();
  h.runner.gatewayApprovals = new GatewayApprovals({ client: () => g.client, host: h.runner, clock: h.clock, serviceTitle: () => 'Slack' });
  const { thread } = h.runner.create({ text: 'Post to Slack', accountId: 'claude-3' });
  const s = h.adapter.last;
  s.init();
  return { h, g, s, id: thread.id };
}

test('in a runner: the card shows the call whole, and Allow approves it at the gateway', async () => {
  const { h, g, s, id } = runnerWithGateway();
  s.out({ kind: 'event', event: { type: 'tool.call', itemId: 'c1', name: SEND.name, kind: 'mcp', title: 'slack_send_message', input: SEND.input } });
  g.hold('ap_runner000001', 'slack__slack_send_message', { channel_id: 'U1', message: 'hello' });
  await h.runner.gatewayApprovals!.poll();
  const card = h.events(id).find((e) => e.type === 'approval.requested');
  assert.ok(card, 'a card');
  assert.equal(card.approvalId, 'gw-ap_runner000001');
  assert.equal(card.name, SEND.name);
  assert.equal(card.title, 'Slack: send message');
  assert.deepEqual(card.input, { channel_id: 'U1', message: 'hello' });
  assert.deepEqual(card.options, ['allow', 'deny']);
  assert.match(card.reason, /Connectors gateway is holding/);
  assert.equal(h.runner.get(id).status, 'waiting');
  // The phone can't allow it (it would come through the backend).
  assert.throws(() => h.runner.respondApproval({ threadId: id, approvalId: card.approvalId, decision: 'allow', remote: true }), /on the Mac/);
  await h.runner.respondApproval({ threadId: id, approvalId: card.approvalId, decision: 'allow' });
  assert.equal(g.approvals.get('ap_runner000001')!.status, 'approved');
  const done = h.events(id).find((e) => e.type === 'approval.resolved' && e.approvalId === card.approvalId);
  assert.equal(done?.decision, 'allow');
  assert.notEqual(h.runner.get(id).status, 'waiting');
  h.runner.gatewayApprovals!.stop();
});

test('in a runner: Deny declines it, and the card outlives the turn', async () => {
  const { h, g, s, id } = runnerWithGateway();
  s.out({ kind: 'event', event: { type: 'tool.call', itemId: 'c1', name: SEND.name, kind: 'mcp', title: 'slack_send_message', input: SEND.input } });
  g.hold('ap_runner000002', 'slack__slack_send_message', { channel_id: 'U1', message: 'hello' });
  await h.runner.gatewayApprovals!.poll();
  s.result('completed');
  await tick();
  const open = h.events(id).filter((e) => e.type === 'approval.resolved');
  assert.equal(open.length, 0, 'still open after the turn');
  await h.runner.respondApproval({ threadId: id, approvalId: 'gw-ap_runner000002', decision: 'deny' });
  assert.equal(g.approvals.get('ap_runner000002')!.status, 'declined');
  const done = h.events(id).find((e) => e.type === 'approval.resolved');
  assert.equal(done?.decision, 'deny');
  h.runner.gatewayApprovals!.stop();
});

test('in a runner: decided on the page first, Allow just closes the card as it went', async () => {
  const { h, g, s, id } = runnerWithGateway();
  s.out({ kind: 'event', event: { type: 'tool.call', itemId: 'c1', name: SEND.name, kind: 'mcp', title: 'slack_send_message', input: SEND.input } });
  g.hold('ap_runner000003', 'slack__slack_send_message', { channel_id: 'U1', message: 'hello' });
  await h.runner.gatewayApprovals!.poll();
  g.approvals.get('ap_runner000003')!.status = 'declined';
  await h.runner.respondApproval({ threadId: id, approvalId: 'gw-ap_runner000003', decision: 'allow' });
  assert.equal(g.approvals.get('ap_runner000003')!.status, 'declined');
  const done = h.events(id).find((e) => e.type === 'approval.resolved');
  assert.equal(done?.decision, 'deny');
  h.runner.gatewayApprovals!.stop();
});

void POLL_MS;
