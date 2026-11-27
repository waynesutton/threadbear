// The website's HTTP routes (docs/website.md). Everything is the static
// site from the static-hosting component, except a shared thread's page,
// /s/<slug>, which is the same index.html with that thread's title, the
// start of its last reply and its first picture in the meta tags, so a
// link pasted into Slack, X or iMessage unfurls as the thread rather than
// as plain "Threadbear". Link-preview bots don't run the page's script, so
// the tags have to be in the HTML itself.
import { httpRouter } from 'convex/server';
import { registerStaticRoutes } from '@convex-dev/static-hosting';
import { components, internal } from './_generated/api';
import { httpAction, type ActionCtx } from './_generated/server';
import type { Id } from './_generated/dataModel';
import { MAX_RUNNER_UPLOAD_BYTES } from './sync';
import { PHONE_UPLOADS } from './phone';
import { sha256 } from './lib';
import { handleMcp, MCP_BODY_MAX } from './mcpRpc';
import { handleHook, HOOK_BODY_MAX } from './hookHttp';
import { takeUpload } from './uploads';
import { safeDecode, withCard } from './shareCard';
import { promoteRoute } from './boxImage';
import { cappedJson } from './httpJson';

const http = httpRouter();

// No other site may put these pages in a frame. Framed out of sight, a
// thread's link and then the Open button of the website's own "Open this in
// another app?" question (web/src/link-confirm.tsx) could be clicked through
// a page laid over them, the address never seen: a shared page (/s/) needs
// no sign-in for that (Daybreak 139, cross-lab review, 17 Nov 2026).
// frame-ancestors has to be a header; a <meta> CSP ignores it.
export const NO_FRAMES = { 'X-Frame-Options': 'DENY', 'Content-Security-Policy': "frame-ancestors 'none'" } as const;

export function noFrames(response: Response): Response {
  const headers = new Headers(response.headers);
  for (const [k, value] of Object.entries(NO_FRAMES)) headers.set(k, value);
  return new Response(response.body, { status: response.status, statusText: response.statusText, headers });
}

http.route({
  pathPrefix: '/s/',
  method: 'GET',
  handler: httpAction(async (ctx, request) => {
    const url = new URL(request.url);
    const slug = safeDecode(url.pathname.slice('/s/'.length).replace(/\/$/, ''));
    const html = await indexHtml(ctx, url);
    if (!html) return new Response('Not Found', { status: 404, headers: { 'Content-Type': 'text/plain' } });
    const card = slug && !slug.includes('/') ? await ctx.runQuery(internal.share.preview, { slug }) : null;
    return new Response(card ? withCard(html, card, url) : html, {
      status: 200,
      headers: {
        'Content-Type': 'text/html; charset=utf-8',
        // Sharing can stop at any time; a cached card would outlive it.
        'Cache-Control': 'no-store',
        'X-Content-Type-Options': 'nosniff',
        ...NO_FRAMES,
      },
    });
  }),
});

// A shared thread's picture: /share/image/<slug>/<id>, the URL share:view
// and the /s/ card give anyone with the link (share.ts sharedImageUrl).
// Served only while the thread is still shared under that link, checked
// at every request (share:shareImageFile), never cached, so stopping
// sharing stops the pictures at once too (Daybreak, 15 Nov 2026).
http.route({
  pathPrefix: '/share/image/',
  method: 'GET',
  handler: httpAction(async (ctx, request) => {
    const gone = () => new Response('Not Found', { status: 404, headers: { 'Content-Type': 'text/plain', 'Cache-Control': 'no-store' } });
    const parts = new URL(request.url).pathname.slice('/share/image/'.length).split('/');
    if (parts.length !== 2) return gone();
    let slug: string;
    let id: string;
    try {
      [slug, id] = parts.map(decodeURIComponent) as [string, string];
    } catch {
      return gone();
    }
    const file = await ctx.runQuery(internal.share.shareImageFile, { slug, id });
    const blob = file ? await ctx.storage.get(file.storageId) : null;
    if (!file || !blob) return gone();
    return new Response(blob, {
      status: 200,
      headers: {
        'Content-Type': file.mediaType,
        'Cache-Control': 'no-store',
        'X-Content-Type-Options': 'nosniff',
        'Content-Security-Policy': "default-src 'none'; sandbox",
        // The page and link previews load it from other origins.
        'Cross-Origin-Resource-Policy': 'cross-origin',
      },
    });
  }),
});

// The push relay asks here whether a /register that named this deployment
// really came from it (push.ts relayInstall, relay/convex/http.ts): it posts
// back the nonce the registration carried, which only this deployment has.
// Answers nothing else, so it tells a caller only whether a guess was right.
http.route({
  path: '/relay/verify',
  method: 'POST',
  handler: httpAction(async (ctx, request) => {
    const b = await cappedJson(request, 1024);
    if (b === 'tooBig') return new Response('request body too large', { status: 413 });
    const nonce = typeof b?.nonce === 'string' && /^[0-9a-f]{48}$/.test(b.nonce) ? b.nonce : null;
    const ok = nonce ? await ctx.runMutation(internal.devices.takeRelayCheck, { nonce }) : false;
    return new Response(JSON.stringify({ ok }), { status: ok ? 200 : 404, headers: { 'Content-Type': 'application/json' } });
  }),
});

// The box-image workflow promoting the image a new box boots, with a GitHub
// Actions OIDC token (boxImage.ts says what it checks).
http.route({ path: '/box-image/promote', method: 'POST', handler: promoteRoute });

// The three upload routes below take their one-use ticket the same way
// (uploads.ts takeUpload): claimed before the body is read, the body read
// no further than the cap, and a file the finish doesn't keep deleted.

// A runner key's uploads (sync:iconUploadUrl hands out this URL with a
// one-use ticket): the file is stored here and recorded as that runner's,
// so its key can attach it, and only it (docs/protocol/keys.md, "Runner keys").
// Answers { storageId } as Convex's own upload URL does, so the runner's
// uploader is the same for both. 429: over an allowance, try again later;
// 403: this ticket won't do.
http.route({
  path: '/runner/upload',
  method: 'POST',
  handler: httpAction(async (ctx, request) =>
    takeUpload(ctx, request, { maxBytes: MAX_RUNNER_UPLOAD_BYTES, claim: internal.sync.claimUpload, finish: internal.sync.finishUpload, refused: 'upload refused: a bad or used ticket, or a revoked key' }),
  ),
});

// A picture the main Mac's runner sends to another runner's thread
// (relay:pictureUploadUrl hands out this URL with a one-use ticket): stored
// here and recorded (relayUploads), so a command can name it and one that
// never went out is found and deleted after a day.
http.route({
  path: '/relay/upload',
  method: 'POST',
  handler: httpAction(async (ctx, request) =>
    takeUpload(ctx, request, { maxBytes: MAX_RUNNER_UPLOAD_BYTES, claim: internal.relay.claimRelayUpload, finish: internal.relay.finishRelayUpload, refused: 'upload refused: a bad or used ticket' }),
  ),
});

// A phone's photo (phone:imageUploadUrl hands out this URL with a one-use
// ticket): stored here and recorded as that phone token's (phoneUploads), so
// only its own commands can send it and only it can discard it, and one
// never sent is found and deleted after a day. Answers { storageId } as
// Convex's own upload URL does, which is what the iPhone app reads.
http.route({
  path: '/phone/upload',
  method: 'POST',
  handler: httpAction(async (ctx, request) =>
    takeUpload(ctx, request, { maxBytes: PHONE_UPLOADS.maxBytes, claim: internal.phone.claimPhoneUpload, finish: internal.phone.finishPhoneUpload, refused: 'upload refused: a bad or used ticket, or a revoked phone' }),
  ),
});

// Outside agents (Grok Bot): Threadbear as a remote MCP server, with an
// agent key (mcp.ts, mcpRpc.ts; docs/convex-backend.md "Outside agents").
// Only internal functions are called, and only after the key's check.
const mcp = httpAction(async (ctx, request) => {
  const headers = request.headers;
  const tooBig = () => new Response(JSON.stringify({ error: 'too big' }), { status: 413, headers: { 'Content-Type': 'application/json' } });
  const length = Number(headers.get('content-length') ?? 0);
  if (request.method === 'POST' && (!Number.isFinite(length) || length > MCP_BODY_MAX)) return tooBig();
  // Read no more than the cap, whatever Content-Length said (or if it said nothing).
  let body = '';
  if (request.method === 'POST' && request.body) {
    const reader = request.body.getReader();
    const chunks: Uint8Array[] = [];
    let size = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > MCP_BODY_MAX) {
        await reader.cancel();
        return tooBig();
      }
      chunks.push(value);
    }
    const all = new Uint8Array(size);
    let at = 0;
    for (const c of chunks) {
      all.set(c, at);
      at += c.byteLength;
    }
    body = new TextDecoder().decode(all);
  }
  const auth = headers.get('authorization') ?? '';
  const presented = /^Bearer\s+(tba_[0-9a-f]{64})$/i.exec(auth.trim())?.[1] ?? '';
  const hash = presented ? await sha256(presented) : '';
  // The caller's address as Convex saw it, never a header the caller can set
  // (X-Forwarded-For would let it pick a new one each try). Without it, bad
  // keys still meet the global limit.
  const meta = await (ctx as { meta?: { getRequestMetadata?: () => Promise<{ ip: string | null }> } }).meta?.getRequestMetadata?.().catch(() => undefined);
  const ip = meta?.ip ?? 'unknown';
  const now = Date.now();
  const out = await handleMcp(
    { method: request.method, header: (n) => headers.get(n), body },
    {
      begin: (tool) => ctx.runMutation(internal.mcp.begin, { hash, tool, ip, now }),
      listThreads: (agent, view) => ctx.runQuery(internal.mcp.listThreads, { agent: agent as never, view }),
      readThread: (agent, threadId, limit) => ctx.runQuery(internal.mcp.readThread, { agent: agent as never, threadId, ...(limit === undefined ? {} : { limit }) }),
      listPrints: (agent, limit, cursor) => ctx.runQuery(internal.mcp.listPrints, { agent: agent as never, ...(limit === undefined ? {} : { limit }), ...(cursor === undefined ? {} : { cursor }) }),
      readPrint: (agent, slug, offset, limit) => ctx.runQuery(internal.mcp.readPrint, { agent: agent as never, slug, ...(offset === undefined ? {} : { offset }), ...(limit === undefined ? {} : { limit }) }),
      startThread: (agent, text) => ctx.runMutation(internal.mcp.startThread, { agent: agent as never, text }),
      sendMessage: (agent, threadId, text) => ctx.runMutation(internal.mcp.sendMessage, { agent: agent as never, threadId, text }),
    },
  );
  return new Response(out.body ?? null, { status: out.status, headers: out.headers });
});
http.route({ path: '/mcp', method: 'POST', handler: mcp });
http.route({ path: '/mcp', method: 'GET', handler: mcp });
http.route({ path: '/mcp', method: 'DELETE', handler: mcp });

// Webhooks (webhooks.ts, hookHttp.ts): POST /hook/<key> starts a thread or
// messages one. Exact routes still win over this prefix. The body is read
// no further than the cap, whatever Content-Length says.
const hook = httpAction(async (ctx, request) => {
  const headers = request.headers;
  const tooBig = () => new Response(JSON.stringify({ error: 'too big' }), { status: 413, headers: { 'Content-Type': 'application/json' } });
  const length = Number(headers.get('content-length') ?? 0);
  if (request.method === 'POST' && (!Number.isFinite(length) || length > HOOK_BODY_MAX)) return tooBig();
  let body = '';
  if (request.method === 'POST' && request.body) {
    const reader = request.body.getReader();
    const chunks: Uint8Array[] = [];
    let size = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > HOOK_BODY_MAX) {
        await reader.cancel();
        return tooBig();
      }
      chunks.push(value);
    }
    const all = new Uint8Array(size);
    let at = 0;
    for (const c of chunks) {
      all.set(c, at);
      at += c.byteLength;
    }
    body = new TextDecoder().decode(all);
  }
  // Convex's own view of the caller's address, never a header it can set.
  const meta = await (ctx as { meta?: { getRequestMetadata?: () => Promise<{ ip: string | null }> } }).meta?.getRequestMetadata?.().catch(() => undefined);
  const ip = meta?.ip ?? 'unknown';
  const now = Date.now();
  const out = await handleHook({ method: request.method, path: new URL(request.url).pathname, header: (n) => headers.get(n), body }, (a) =>
    ctx.runMutation(internal.webhooks.deliver, { hash: a.hash, ip, now, body: a.body, contentType: a.contentType, ...(a.dedupe ? { dedupe: a.dedupe } : {}) }),
  );
  return new Response(out.body, { status: out.status, headers: out.headers });
});
http.route({ pathPrefix: '/hook/', method: 'POST', handler: hook });

// The profile page (profile.ts) at /profile, and at the front page when
// Mike has it shown there: index.html with the page's name, bio and picture
// in the meta tags, so a link to his site unfurls as him, as /s/ does for a
// shared thread. Exact routes, which Convex matches before any path prefix,
// so the static routes below go on serving everything else. At / the page
// itself still decides: signed in, Mike gets his threads.
for (const path of ['/profile', '/'] as const) {
  http.route({
    path,
    method: 'GET',
    handler: httpAction(async (ctx, request) => {
      const url = new URL(request.url);
      const html = await indexHtml(ctx, url);
      if (!html) return new Response('Not Found', { status: 404, headers: { 'Content-Type': 'text/plain' } });
      const card = await ctx.runQuery(internal.profile.card, { home: path === '/' });
      return new Response(card ? withCard(html, card, url, 'profile') : html, {
        status: 200,
        // Publishing can stop at any time; a cached card would outlive it.
        headers: { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff', ...NO_FRAMES },
      });
    }),
  });
}

// The site's own pages go out with NO_FRAMES too. registerStaticRoutes has
// no option for headers, so it registers onto this stand-in router, which
// wraps each handler it's given (an httpAction's _handler is the function it
// was made from).
registerStaticRoutes(
  {
    route: (spec: Parameters<typeof http.route>[0]) => {
      const inner = (spec.handler as unknown as { _handler?: (ctx: unknown, request: Request) => Promise<Response> })._handler;
      // A newer Convex without _handler fails the deploy here, rather than
      // serving pages without the header.
      if (typeof inner !== 'function') throw new Error("static-hosting's handler can't be wrapped: NO_FRAMES needs another way in");
      http.route({ ...spec, handler: httpAction(async (ctx, request) => noFrames(await inner(ctx, request))) });
    },
  } as unknown as typeof http,
  components.staticHosting,
);

export default http;

// The site's index.html, from whichever of the three places the component
// keeps files (registerStaticRoutes reads them the same way).
async function indexHtml(ctx: ActionCtx, url: URL): Promise<string | null> {
  const asset = await ctx.runQuery(components.staticHosting.lib.resolveAssetForHttp, { path: '/index.html' });
  if (asset?.blobId) return text(fetch(`${url.origin}/fs/blobs/${asset.blobId}`));
  if (asset?.storageUrl) return text(fetch(asset.storageUrl));
  if (asset?.appStorageId) return (await (await ctx.storage.get(asset.appStorageId as Id<'_storage'>))?.text()) ?? null;
  return null;
}

async function text(response: Promise<Response>): Promise<string | null> {
  const r = await response;
  return r.ok ? r.text() : null;
}
