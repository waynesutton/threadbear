// Whether a threadbear tool call may run, for adapters whose CLI doesn't ask
// before a client-side tool itself (Codex's dynamic tools, an ACP agent's
// MCP server). delete_thread always asks; in any mode but Full, so does
// anything that changes something, unless "Always allow" covers it.
import { ALWAYS_ASK_TOOLS, NEVER_ASK_TOOLS } from '../threadbear-mcp.ts';
import { cardInput, tooLongForCard, toolTitle } from '../tools.ts';
import type { PermissionMode } from '../protocol.ts';
import type { ToolApprovalAnswer, ToolApprovalRequest } from './types.ts';

export type ToolGateDeps = {
  requestApproval(req: ToolApprovalRequest): Promise<ToolApprovalAnswer>;
  cwd(): string;
  // The thread's permission mode now (not the turn's).
  mode(): PermissionMode;
  // The running turn is an Ask one: only the tools that read or never ask
  // run, whatever the mode (docs/protocol.md, "Ask mode").
  readOnly?(): boolean;
  // "Always allow" rules: a tool's full name, as for Claude.
  rules(): Iterable<string>;
  addRules(rules: string[]): void;
};

export type ToolGateCall = {
  fullName: string; // mcp__threadbear__<tool>
  readOnlyHint?: boolean;
  args: Record<string, unknown>;
  toolUseId?: string;
  signal: AbortSignal;
};

// { allowed: false, message } is said to the agent as the tool's result.
export async function gateThreadbearTool(deps: ToolGateDeps, call: ToolGateCall): Promise<{ allowed: true } | { allowed: false; message: string }> {
  const { fullName, args } = call;
  const alwaysAsk = ALWAYS_ASK_TOOLS.has(fullName);
  const readOnly = NEVER_ASK_TOOLS.has(fullName) || call.readOnlyHint === true;
  const allowed = !alwaysAsk && [...deps.rules()].includes(fullName);
  if (!readOnly && deps.readOnly?.()) {
    const name = fullName.replace(/^mcp__[^_]+__/, '');
    return { allowed: false, message: `This turn is in Ask mode, so nothing may change: ${name} is off for it. Answer from what you can read, and say what you'd do rather than doing it.` };
  }
  if (!alwaysAsk && (deps.mode() === 'full' || readOnly || allowed)) return { allowed: true };
  // All of it or nothing: an allow runs the whole of what it allowed, so a
  // card cut short would approve words nobody saw.
  const card = cardInput(args, 'all');
  if (card.tooLong) return { allowed: false, message: tooLongForCard(card.tooLong) };
  const ans = await deps.requestApproval({
    toolUseId: call.toolUseId,
    name: fullName,
    title: toolTitle(fullName, args, deps.cwd()),
    input: card.input,
    canAlwaysAllow: !alwaysAsk,
    signal: call.signal,
  });
  if (ans.decision !== 'allow' && ans.decision !== 'allowAlways') {
    return { allowed: false, message: ans.message || (ans.decision === 'expired' ? 'The request expired before the user answered.' : 'The user denied this. Ask them how to proceed.') };
  }
  if (ans.decision === 'allowAlways' && !alwaysAsk) deps.addRules([fullName]);
  return { allowed: true };
}
