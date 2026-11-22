// A quiet inbox (docs/protocol/threads.md, "A quiet inbox"): a finished thread
// another thread started, with nothing for Mike, archives itself.
//
// Mike, 12 Nov 2026, of three threads other threads had started (a bun fix
// split off a Let's Build It thread, two GPUI fixes from the port's
// thread): "there's nothing for me to do... it should just auto archive."
// Tried as an experiment for four days (about 70 helpers archived, none
// brought back), then made standard on 16 Nov. It also hibernated a thread
// whose only loose ends were threads it started; that was taken out then,
// as it hid threads Mike wanted to read (a starred coordinator, the daily
// brief) and its wakes rarely had anything to say.
//
// Each method's `this` is the Runner; runner.ts puts them on its prototype.
import type { Runner, ThreadState } from '../runner.ts';
import type { RunnerPart } from './part.ts';
import type { LooseEnd } from '../protocol.ts';
import { sentByAgent } from '../protocol.ts';
import { completionOpen } from '../completion.ts';
import { experimentOn } from '../experiments.ts';

export const HELPER_ARCHIVED_NOTE = 'Archived: another thread started this one, and it finished with nothing left for you.';

export const quietInboxPart = {
  // After a turn that finished for good (completeTurn's `finished`, with no
  // review of it to come). `next`: whether it suggested follow-ups, which
  // are Mike's to tick or leave.
  async settleQuietly(this: Runner, st: ThreadState, next: boolean, quiet = false): Promise<void> {
    const { rec, rt } = st;
    if (experimentOn(this.cfg.home, 'clearerCompletion') && completionOpen(rec.completion)) return;
    // A bear's unattended run that said it had nothing new (end_turn's
    // quiet) goes too; any other bear report stays, even a helper's.
    const bear = experimentOn(this.cfg.home, 'bears') && Boolean(this.bearOf?.(rec.cwd));
    const unattended = Boolean(rec.schedule || ['thread', 'schedule', 'tool'].includes(rec.startedBy?.kind ?? ''));
    const quietBear = bear && unattended && quiet;
    if (bear && !quietBear) return;
    // Cheap checks first, so a thread that can't be archived this way
    // never runs git.
    if (next || !this.helperNobodyTouched(st, quietBear)) return;
    const seq = this.looseEndsCard(st);
    if (seq === undefined) return;
    let ends: LooseEnd[];
    try {
      // The done card's own check of this card, if it's going (looseEnds).
      ends = await (rt.looseEndsGather ?? this.gatherLooseEnds(st));
    } catch (e) {
      // Not known to be clean, so it stays where Mike can see it.
      this.log.warn("couldn't check a finished thread's loose ends", { threadId: rec.id, error: String(e) });
      return;
    }
    // Git took a moment. Anything since (a message, a new turn, Mike or a
    // schedule putting it away, the runner stopping) decides instead.
    if (rt.deleted || this.stopping || rt.turn || rec.queue.length || this.looseEndsCard(st) !== seq || this.bucket(rec) !== 'inbox') return;
    // Anything left open, or a check-in it booked (that brings it back by
    // itself), keeps it in the inbox.
    delete rt.quietOnceChildrenDone;
    if (ends.some((e) => e.kind !== 'thread')) return;
    // Threads it started still going (as they stand now, rather than when
    // git began) keep it there until they're done: each one finishing
    // looks again (recheckStarter), while this card is still its last.
    if (this.childEnds(st).length) {
      rt.quietOnceChildrenDone = seq;
      return;
    }
    if (!this.helperNobodyTouched(st, quietBear)) return;
    this.emit(st, { type: 'system.note', text: quietBear ? 'Archived: this bear finished its unattended run with nothing new to report.' : HELPER_ARCHIVED_NOTE });
    this.update({ threadId: rec.id, archived: true, archivedBy: 'threadbear' });
    this.log.info('quiet inbox: archived a finished helper thread', { threadId: rec.id, startedBy: rec.startedBy?.threadId });
  },

  // One another thread's agent started that Mike has had nothing to do
  // with: never written in (answering its card counts), starred, or taken
  // back out of the archive. A tool's, a schedule's or the librarian's is
  // often the thing he wants to read, and has no thread holding its result.
  helperNobodyTouched(this: Runner, st: ThreadState, unattendedBear = false): boolean {
    const { rec } = st;
    if ((!unattendedBear && rec.startedBy?.kind !== 'thread') || rec.starredAt || rec.keptOut || rec.reviewOf) return false;
    // Its first message is its starter's brief, whatever it looks like (`tb
    // new` from an agent's shell has no "Sent by the agent" line, so
    // rec.asked can't be trusted for it). Anything of Mike's after that
    // makes it his.
    const log = this.store.log(rec.id);
    const events = log.read(1, log.lastSeq);
    if (unattendedBear && events.some((e) => e.type === 'approval.resolved' && e.decision !== 'expired')) return false;
    if (unattendedBear) {
      // A final empty check must not hide an earlier report in this shared
      // worker. Every completed turn in it must have explicitly been quiet.
      const quietTurns = new Map<string, boolean>();
      for (const e of events) {
        if (!e.turnId) continue;
        if (e.type === 'tool.call' && e.name.split('__').at(-1) === 'end_turn') quietTurns.set(e.turnId, e.input?.quiet === true);
        if (e.type === 'turn.completed' && (e.outcome !== 'completed' || !quietTurns.get(e.turnId))) return false;
      }
    }
    const said = events.filter((e) => e.type === 'user.message');
    return !said.slice(1).some((m) => (!m.origin || m.origin === 'phone') && !sentByAgent(m.text));
  },
};

export type QuietInboxPart = RunnerPart<typeof quietInboxPart>;
