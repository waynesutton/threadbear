// The network probe through a proxy (src/net-probe.ts) against a local
// server standing in for one. Daybreak, 8 Nov ("Unbounded proxy probe
// responses can exhaust runner resources"): a proxy answering CONNECT with a
// steady stream and no line end kept one probe going, its buffer growing,
// until the runner ran out of memory; and an https:// proxy got a plain
// connection.
import { test, type TestContext } from 'node:test';
import assert from 'node:assert/strict';
import net from 'node:net';
import { makeReachable, probeTarget, PROXY_HEAD_MAX } from '../../src/net-probe.ts';

function proxy(onConnect: (sock: net.Socket, first: Buffer) => void): Promise<{ port: number; close: () => void; firstBytes: Buffer[] }> {
  const firstBytes: Buffer[] = [];
  const socks = new Set<net.Socket>();
  const server = net.createServer((sock) => {
    socks.add(sock);
    sock.on('error', () => {});
    sock.once('data', (b: Buffer) => {
      firstBytes.push(b);
      onConnect(sock, b);
    });
  });
  return new Promise((resolve) =>
    server.listen(0, '127.0.0.1', () =>
      resolve({
        port: (server.address() as net.AddressInfo).port,
        firstBytes,
        close: () => {
          for (const s of socks) s.destroy();
          server.close();
        },
      }),
    ),
  );
}

// Scheduling delays are not probe failures. Control time and socket input
// for the bounds; the tests below still exercise real TCP and TLS sockets.
function controlledProxy(t: TestContext) {
  const socket = new net.Socket();
  t.after(() => socket.destroy());
  t.mock.method(net, 'connect', () => socket);
  t.mock.method(socket, 'write', () => true);
  // An active stream never fires the socket's inactivity timeout.
  t.mock.method(socket, 'setTimeout', () => socket);
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const result = makeReachable({ env: { HTTPS_PROXY: 'http://proxy.example' }, timeoutMs: 400 })('claude');
  socket.emit('connect');
  return { socket, result };
}

test('a proxy sending a steady stream with no line end is given up on by the deadline, not kept', async (t) => {
  const { socket, result } = controlledProxy(t);
  let answer: boolean | undefined;
  void result.then((ok) => { answer = ok; });
  // Each chunk is below the cap, and all of them together are too.
  for (let i = 0; i < 19; i++) {
    t.mock.timers.tick(20);
    socket.emit('data', Buffer.from('A'));
  }
  t.mock.timers.tick(19);
  await Promise.resolve();
  assert.equal(answer, undefined, 'still waiting immediately before the deadline');
  assert.equal(socket.destroyed, false);
  t.mock.timers.tick(1);
  await Promise.resolve();
  assert.equal(answer, false, 'the absolute deadline ends an active stream');
  assert.equal(socket.destroyed, true);
});

test('a proxy answer longer than a status line without its end is refused at once, and not kept', async (t) => {
  const { socket, result } = controlledProxy(t);
  let answer: boolean | undefined;
  void result.then((ok) => { answer = ok; });
  socket.emit('data', Buffer.alloc(PROXY_HEAD_MAX, 0x41));
  await Promise.resolve();
  assert.equal(answer, undefined, 'the cap itself is allowed');
  socket.emit('data', Buffer.alloc(4 * 1024 * 1024, 0x41));
  await Promise.resolve();
  assert.equal(answer, false, 'the cap ends the probe without advancing its deadline');
  assert.equal(socket.destroyed, true);
  assert.ok(PROXY_HEAD_MAX <= 4096);
});

test('a real 200 still says yes, and a 407 no', async () => {
  for (const [line, want] of [['HTTP/1.1 200 Connection established', true], ['HTTP/1.1 407 Proxy Authentication Required', false]] as const) {
    const p = await proxy((sock, first) => {
      assert.match(first.toString('latin1'), /^CONNECT api\.anthropic\.com:443 HTTP\/1\.1\r\n/);
      // In two pieces, as a network may.
      sock.write(line.slice(0, 5));
      setTimeout(() => sock.write(`${line.slice(5)}\r\n\r\n`), 20);
    });
    const ok = await makeReachable({ env: { HTTPS_PROXY: `http://127.0.0.1:${p.port}` }, timeoutMs: 2000 })('claude');
    p.close();
    assert.equal(ok, want, line);
  }
});

test('an https:// proxy is spoken to over TLS, never sent CONNECT in the clear', async () => {
  assert.deepEqual(probeTarget('claude', { HTTPS_PROXY: 'https://proxy.example:8443' }).proxy, { host: 'proxy.example', port: 8443, tls: true });
  assert.equal(probeTarget('claude', { HTTPS_PROXY: 'http://proxy.example' }).proxy?.tls, false);
  // A plain server where the proxy should be: it gets a TLS hello, answers
  // as a proxy would in the clear, and the probe says no.
  const p = await proxy((sock) => sock.write('HTTP/1.1 200 Connection established\r\n\r\n'));
  const ok = await makeReachable({ env: { HTTPS_PROXY: `https://127.0.0.1:${p.port}` }, timeoutMs: 2000 })('claude');
  p.close();
  assert.equal(ok, false);
  assert.equal(p.firstBytes[0]?.[0], 0x16, 'a TLS handshake record');
  assert.ok(!p.firstBytes[0]?.toString('latin1').startsWith('CONNECT'));
});
