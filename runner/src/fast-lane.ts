// Fast lane (experiment `fastLane`): read-only quick web lookup that answers
// in a couple of seconds without spawning a coding CLI, loading skills or
// touching local files. Calls `treg__call` (`exa.web.answer`) through the
// local Connectors gateway (reusing the `helperForAgent` pattern from
// linear-tasks.ts / notion-tasks.ts), with an optional direct Exa fallback
// configured in `runner.json`.
import {
  DEFAULT_FAST_LANE_CHOICE,
  fastLaneOrder,
  fastLaneTitle,
  MIN_STEP_MS,
  parseSearch,
  readWriter,
  SEARCH_TIMEOUT_MS,
  SEARCHES,
  TOTAL_TIMEOUT_MS,
  WRITER_TIMEOUT_MS,
  writerPrompt,
  type FastLaneChoice,
  type FastSource,
  type FastWriter,
} from './fast-lane-models.ts';
import type { Logger } from './log.ts';
import { waitingFor } from './linear-tasks.ts';
import { helperForAgent, helperHeaders, parseRpc } from './notion-tasks.ts';
import { cutSafely } from './safe-cut.ts';
import { credentialUrlProblem } from './secure-url.ts';

export const FAST_LANE_EXPERIMENT = 'fastLane';
export const DEFAULT_FAST_LANE_ENDPOINT = 'exa.web.answer';
export const DEFAULT_FAST_LANE_TIMEOUT_MS = 15_000;
export const DEFAULT_EXA_ANSWER_URL = 'https://api.exa.ai/answer';
const MIN_TIMEOUT_MS = 2_000;
const MAX_TIMEOUT_MS = 60_000;
const MAX_CITATIONS = 20;
const MAX_HANDOFF_CHARS = 2_500;

export type FastLaneConfig = {
  // Who answers (fast-lane-models.ts); Exa when left out.
  model?: FastLaneChoice;
  timeoutMs?: number;
  endpointId?: string;
  headersHelper?: string;
  exaApiKey?: string;
  exaUrl?: string;
};

export type FastCitation = {
  title: string;
  url: string;
  // Its places in the list Exa gave, which the answer's [n] markers count by.
  // More than one when Exa listed the same page twice.
  numbers: number[];
  publishedDate?: string;
};

export type FastLookupResult = {
  answer: string;
  citations: FastCitation[];
  source: string;
  durationMs: number;
};

export type FastLookup = (query: string, opts?: { signal?: AbortSignal }) => Promise<FastLookupResult>;

export type ParsedFastQuery =
  | { fast: true; query: string; viaPrefix: boolean }
  | { fast: false };

const WEB_PREFIX = /^\/web(?:\s+([\s\S]*))?$/i;

/** Detects whether a turn should use the fast lane (`fast: true` or `/web <query>`). */
export function parseFastQuery(text: string, fastFlag = false): ParsedFastQuery {
  const trimmed = text.trim();
  const match = WEB_PREFIX.exec(trimmed);
  if (fastFlag) {
    const query = match ? (match[1] ?? '').trim() : trimmed;
    return { fast: true, query, viaPrefix: false };
  }
  if (match && match[1] && match[1].trim()) {
    return { fast: true, query: match[1].trim(), viaPrefix: true };
  }
  return { fast: false };
}

/** Connectors login helper for Fast lane: override, else the `threadbear` agent login. */
export function fastLaneHelper(mainHelper: string, override?: string): string | undefined {
  if (override?.trim()) return override.trim();
  return helperForAgent(mainHelper, 'threadbear');
}

function clampTimeout(ms: number | undefined): number {
  if (typeof ms !== 'number' || !Number.isFinite(ms)) return DEFAULT_FAST_LANE_TIMEOUT_MS;
  return Math.max(MIN_TIMEOUT_MS, Math.min(MAX_TIMEOUT_MS, Math.floor(ms)));
}

function combinedSignal(timeoutMs: number, external?: AbortSignal): AbortSignal {
  const timer = AbortSignal.timeout(timeoutMs);
  return external ? AbortSignal.any([external, timer]) : timer;
}

function isAbortError(err: unknown): boolean {
  return err instanceof Error && (err.name === 'AbortError' || err.name === 'TimeoutError');
}

/** treg can't be reached at all (signed out, waiting for approval), so no other call through it is worth trying. */
class TregDown extends Error {}

/** The gateway leaves a signed-out service's tools out, so the call comes back as an unknown tool. */
function connectorsError(message: string, mcpUrl: string): Error {
  if (!/unknown tool:\s*treg__call\b/i.test(message)) return new Error(message);
  const page = `${new URL(mcpUrl).origin}/admin/connect/treg`;
  return new TregDown(`Fast web search can't reach treg. Sign in to it in Connectors (${page}), then send this again.`);
}

type Json = Record<string, unknown>;

function asObject(value: unknown): Json | undefined {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as Json) : undefined;
}

function tryJson(text: string): unknown {
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return undefined;
  }
}

function unwrapPayload(raw: unknown): Json | undefined {
  let current = asObject(raw);
  for (let depth = 0; depth < 3 && current; depth++) {
    if (typeof current.answer === 'string' || Array.isArray(current.citations) || Array.isArray(current.results)) {
      return current;
    }
    const next = current.body ?? current.data ?? current.response ?? current.result;
    if (typeof next === 'string') {
      current = asObject(tryJson(next));
    } else {
      current = asObject(next);
    }
  }
  return current;
}

function formatIsoDate(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  const iso = /^(\d{4}-\d{2}-\d{2})/.exec(trimmed);
  if (iso) return iso[1];
  const parsed = Date.parse(trimmed);
  if (Number.isNaN(parsed)) return undefined;
  return new Date(parsed).toISOString().slice(0, 10);
}

function extractCitations(payload: Json): FastCitation[] {
  const list = Array.isArray(payload.citations)
    ? payload.citations
    : Array.isArray(payload.sources)
      ? payload.sources
      : Array.isArray(payload.results)
        ? payload.results
        : [];
  const seen = new Map<string, FastCitation>();
  const out: FastCitation[] = [];
  for (const [index, entry] of list.entries()) {
    const item = asObject(entry);
    if (!item) continue;
    const url = typeof item.url === 'string' ? item.url.trim() : '';
    if (!url || !/^https?:\/\//i.test(url) ) continue;
    const earlier = seen.get(url);
    if (earlier) {
      earlier.numbers.push(index + 1);
      continue;
    }
    if (out.length >= MAX_CITATIONS) continue;
    const rawTitle = typeof item.title === 'string' && item.title.trim() ? item.title.trim() : url;
    const publishedDate = formatIsoDate(item.publishedDate ?? item.published_date ?? item.date);
    const citation = { title: rawTitle, url, numbers: [index + 1], ...(publishedDate ? { publishedDate } : {}) };
    seen.set(url, citation);
    out.push(citation);
  }
  return out;
}

/** Parses `exa.web.answer` JSON (or wrapped `treg__call` responses) into answer + dated citations. */
export function parseExaAnswer(rawText: string): { answer: string; citations: FastCitation[] } {
  const trimmed = rawText.trim();
  if (!trimmed) throw new Error('Fast web search returned an empty answer.');
  const parsed = tryJson(trimmed);
  const payload = unwrapPayload(parsed);
  if (!payload) {
    return { answer: trimmed, citations: [] };
  }
  const answer = typeof payload.answer === 'string' && payload.answer.trim()
    ? payload.answer.trim()
    : typeof payload.text === 'string' && payload.text.trim()
      ? payload.text.trim()
      : '';
  const citations = extractCitations(payload);
  if (!answer && citations.length === 0) {
    throw new Error('Fast web search returned no answer.');
  }
  return { answer: answer || 'Here are the top matching sources:', citations };
}

/** Formats the fast-lane reply markdown with compact dated citation links. */
export function formatFastAnswer(answer: string, citations: FastCitation[]): string {
  const body = answer.trim();
  if (!citations.length) return body;
  // Bullets with the marker written out, not a Markdown list: Markdown would
  // renumber past a gap. An answer with no markers gets plain bullets.
  const marked = /\[\d+\]/.test(body);
  const links = citations.map((c) => {
    const cleanTitle = c.title.replace(/[[\]]/g, '').replace(/\s+/g, ' ').trim() || c.url;
    const date = c.publishedDate ? ` (${c.publishedDate})` : '';
    const marker = marked ? `${c.numbers.map((n) => `[${n}]`).join('')} ` : '';
    return `- ${marker}[${cleanTitle}](${c.url})${date}`;
  });
  return `${body}\n\n${links.join('\n')}`;
}

/** Compact system note shown under a fast-lane reply on both GPUI and iPhone. */
export function formatFastNote(durationMs: number, source = 'Exa'): string {
  const secs = Math.max(0.1, durationMs / 1000).toFixed(1);
  return `Fast web search · ${secs}s · ${source}`;
}

/** Compact handoff line injected before the next coding turn when a live session follows a fast lookup. */
export function formatFastHandoff(question: string, answer: string): string {
  const q = cutSafely(question.trim(), 300);
  const a = cutSafely(answer.trim(), MAX_HANDOFF_CHARS);
  return (
    `[Threadbear fast web search context] Between coding turns, the user ran a read-only fast web lookup.\n` +
    `Question: ${q}\n` +
    `Answer:\n${a}\n` +
    `[End of fast web search context. The new message follows.]\n\n`
  );
}

export type FastLaneLookupOptions = {
  connectors?: {
    mcpUrl: string;
    headers: () => Promise<Record<string, string>>;
    fallback?: () => Promise<Record<string, string>>;
  };
  endpointId?: string;
  exa?: {
    apiKey: string;
    url?: string;
  };
  timeoutMs?: number;
  // Read for each question, so a new choice takes effect at once.
  choice?: () => FastLaneChoice;
  writers?: Partial<Record<FastLaneChoice, FastWriter>>;
  fetch?: typeof fetch;
  log?: Logger;
  now?: () => number;
};

/**
 * Creates a read-only Fast lane lookup function.
 * Primary: the chosen answerer (fast-lane-models.ts), then the others in turn.
 * Exa is the local Connectors gateway's `treg__call` (`exa.web.answer`).
 * Optional fallback: direct Exa Answer API when `fastLane.exaApiKey` is configured in `runner.json`.
 */
export function createFastLaneLookup(opts: FastLaneLookupOptions): FastLookup {
  const f = opts.fetch ?? fetch;
  const now = opts.now ?? Date.now;
  const timeoutMs = clampTimeout(opts.timeoutMs);
  const endpointId = opts.endpointId?.trim() || DEFAULT_FAST_LANE_ENDPOINT;
  let rpcId = 0;
  let useMainHelper = false;

  // One treg endpoint through the Connectors gateway; its reply's text.
  async function callTreg(endpoint: string, params: Record<string, unknown>, ms: number, signal?: AbortSignal): Promise<string> {
    const conn = opts.connectors;
    if (!conn) throw new Error('Connectors is not configured on this Mac.');
    const problem = credentialUrlProblem(conn.mcpUrl);
    if (problem) throw new Error(`The Connectors gateway URL ${problem}.`);

    const loadHeaders = async (): Promise<Record<string, string>> => {
      if (!useMainHelper) {
        try {
          return await conn.headers();
        } catch (err) {
          if (!conn.fallback) throw err;
          useMainHelper = true;
          opts.log?.warn('fast-lane: no Threadbear login for Connectors, falling back to main login');
        }
      }
      return conn.fallback!();
    };

    const post = async (hdrs: Record<string, string>) =>
      f(conn.mcpUrl, {
        method: 'POST',
        headers: { ...hdrs, 'content-type': 'application/json', accept: 'application/json, text/event-stream' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: ++rpcId,
          method: 'tools/call',
          params: {
            name: 'treg__call',
            arguments: {
              endpoint_id: endpoint,
              params,
            },
          },
        }),
        redirect: 'error',
        signal: combinedSignal(ms, signal),
      });

    let res = await post(await loadHeaders());
    if (res.status === 401 && !useMainHelper && conn.fallback) {
      useMainHelper = true;
      res = await post(await conn.fallback());
    }
    // Connectors itself refusing: every other treg call would be refused too.
    if (res.status === 401 || res.status === 403) throw new TregDown(`Connectors gateway answered ${res.status}.`);
    if (!res.ok) throw new Error(`Connectors gateway answered ${res.status}.`);
    const msg = parseRpc(await res.text());
    if (msg.error) throw connectorsError(msg.error.message ?? 'Connectors gateway returned an error.', conn.mcpUrl);
    const text = (msg.result?.content ?? [])
      .filter((c) => c.type === 'text')
      .map((c) => c.text ?? '')
      .join('\n');
    if (msg.result?.isError) throw connectorsError(text.slice(0, 300) || 'Fast web search failed at Connectors.', conn.mcpUrl);
    const held = waitingFor(text);
    if (held) {
      throw new TregDown(`Connectors asked for approval before running fast web search (${held.url}).`);
    }
    return text;
  }

  async function callDirectExa(query: string, ms: number, signal?: AbortSignal): Promise<{ answer: string; citations: FastCitation[] }> {
    const exa = opts.exa;
    if (!exa?.apiKey) throw new Error('No direct Exa fallback key is configured.');
    const url = exa.url?.trim() || DEFAULT_EXA_ANSWER_URL;
    const problem = credentialUrlProblem(url);
    if (problem) throw new Error(`The Exa API URL ${problem}.`);
    const res = await f(url, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        accept: 'application/json',
        'x-api-key': exa.apiKey,
      },
      body: JSON.stringify({ query, text: true }),
      redirect: 'error',
      signal: combinedSignal(ms, signal),
    });
    if (!res.ok) throw new Error(`Exa answered ${res.status}.`);
    return parseExaAnswer(await res.text());
  }

  // Exa: treg's, else the direct key.
  async function askExa(query: string, ms: () => number, signal: AbortSignal | undefined, tregDown: () => boolean): Promise<FastAnswer> {
    let first: unknown;
    if (opts.connectors && !tregDown()) {
      try {
        return { ...parseExaAnswer(await callTreg(endpointId, { query, text: true }, ms(), signal)), source: 'Exa' };
      } catch (err) {
        if (signal?.aborted) throw err;
        first = err;
        opts.log?.warn('fast-lane: Connectors lookup failed', { error: err instanceof Error ? err.message : String(err) });
      }
    }
    // The direct key gets its own go however treg failed.
    if (opts.exa?.apiKey) return { ...(await callDirectExa(query, ms(), signal)), source: 'Exa direct' };
    throw first ?? new Error('Fast web search needs local Connectors (treg) or fastLane.exaApiKey in runner.json.');
  }

  async function search(query: string, ms: () => number, signal: AbortSignal | undefined): Promise<FastSource[]> {
    let last: unknown = new Error('Connectors is not configured on this Mac.');
    for (const s of opts.connectors ? SEARCHES : []) {
      if (ms() < MIN_STEP_MS) break;
      try {
        const found = parseSearch(await callTreg(s.endpointId, s.params(query), Math.min(SEARCH_TIMEOUT_MS, ms()), signal));
        if (found.length) return found;
        last = new Error('The web search found nothing.');
      } catch (err) {
        if (signal?.aborted || err instanceof TregDown) throw err;
        last = err;
        opts.log?.warn('fast-lane: search failed', { endpoint: s.endpointId, error: err instanceof Error ? err.message : String(err) });
      }
    }
    throw last;
  }

  return async (query: string, callOpts?: { signal?: AbortSignal }): Promise<FastLookupResult> => {
    const clean = query.trim();
    if (!clean) throw new Error('Fast web search needs a question.');
    const startedAt = now();
    const signal = callOpts?.signal;
    const choice = opts.choice?.() ?? DEFAULT_FAST_LANE_CHOICE;
    const order = fastLaneOrder(choice).filter((c) => c === 'exa' || opts.writers?.[c]);
    // With someone to step in, Exa isn't waited on for its full time.
    const exaMs = order.length > 1 && opts.timeoutMs === undefined ? WRITER_TIMEOUT_MS : timeoutMs;
    const totalMs = Math.max(TOTAL_TIMEOUT_MS, exaMs);
    const left = () => totalMs - (now() - startedAt);
    // One search for however many models get asked.
    let sources: Promise<FastSource[]> | undefined;
    let down = false;
    const errors: { who: FastLaneChoice; err: unknown }[] = [];

    for (const who of order) {
      if (errors.length && left() < MIN_STEP_MS) break;
      try {
        const answer =
          who === 'exa'
            ? await askExa(clean, () => (errors.length ? Math.max(1, Math.min(exaMs, left())) : exaMs), signal, () => down)
            : await write(who);
        const instead = who !== choice && opts.writers ? ` (${fastLaneTitle(choice)} didn't answer)` : '';
        return { ...answer, source: `${answer.source}${instead}`, durationMs: Math.max(0, now() - startedAt) };
      } catch (err) {
        if (signal?.aborted) throw err;
        if (err instanceof TregDown) down = true;
        errors.push({ who, err });
        opts.log?.warn('fast-lane: no answer', { who, error: err instanceof Error ? err.message : String(err) });
      }
    }

    async function write(who: FastLaneChoice): Promise<FastAnswer> {
      if (down) throw new Error('No web search to answer from.');
      const found = await (sources ??= search(clean, left, signal));
      const ms = Math.min(WRITER_TIMEOUT_MS, left());
      if (ms < MIN_STEP_MS) throw new Error('Fast web search ran out of time.');
      const text = await within(opts.writers![who]!({ ...writerPrompt(clean, found, new Date(now()).toISOString().slice(0, 10)), timeoutMs: ms }), ms, signal);
      return { ...readWriter(text, found), source: fastLaneTitle(who) };
    }

    // What to do about it (sign in, approve) beats how the last backup failed.
    const err = (errors.find((e) => e.err instanceof TregDown) ?? errors[0])?.err;
    if (isAbortError(err)) throw new Error(`Fast web search timed out after ${Math.round((now() - startedAt) / 1000)}s.`);
    throw err;
  };
}

type FastAnswer = { answer: string; citations: FastCitation[]; source: string };

/** `work`, or a rejection at `ms` or Stop. The model's process ends at its own timeout. */
function within<T>(work: Promise<T>, ms: number, signal?: AbortSignal): Promise<T> {
  const stop = combinedSignal(ms, signal);
  return new Promise<T>((resolve, reject) => {
    const onStop = () => reject(stop.reason instanceof Error ? stop.reason : new Error('Stopped.'));
    if (stop.aborted) return onStop();
    stop.addEventListener('abort', onStop, { once: true });
    work.then(resolve, reject).finally(() => stop.removeEventListener('abort', onStop));
  });
}

/** Builds the live runner's Fast lane lookup from runner config (Connectors + optional runner.json fallback). */
export function fastLaneFromConfig(
  connectorsClient: { mcpUrl: string; headersHelper: string } | undefined,
  fastLaneCfg: FastLaneConfig | undefined,
  log?: Logger,
  models: Pick<FastLaneLookupOptions, 'choice' | 'writers'> = {},
): FastLookup {
  const helperCmd = connectorsClient ? fastLaneHelper(connectorsClient.headersHelper, fastLaneCfg?.headersHelper) : undefined;
  const connectors = connectorsClient
    ? {
        mcpUrl: connectorsClient.mcpUrl,
        headers: helperHeaders(helperCmd ?? connectorsClient.headersHelper),
        ...(helperCmd ? { fallback: helperHeaders(connectorsClient.headersHelper) } : {}),
      }
    : undefined;
  const exa = fastLaneCfg?.exaApiKey?.trim()
    ? { apiKey: fastLaneCfg.exaApiKey.trim(), ...(fastLaneCfg.exaUrl ? { url: fastLaneCfg.exaUrl } : {}) }
    : undefined;
  return createFastLaneLookup({
    ...(connectors ? { connectors } : {}),
    ...(fastLaneCfg?.endpointId ? { endpointId: fastLaneCfg.endpointId } : {}),
    ...(exa ? { exa } : {}),
    ...(fastLaneCfg?.timeoutMs !== undefined ? { timeoutMs: fastLaneCfg.timeoutMs } : {}),
    ...(log ? { log } : {}),
    ...models,
  });
}
