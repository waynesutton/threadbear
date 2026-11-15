# Bears resources

Updated 2026-11-15 04:22 UTC.

The links Wayne supplied while planning and building Threadbear's bears, tables, mentions, avatars, webhooks, memory and unattended runs. Repeated links appear once. The notes describe how we used these sources in November 2026; they are not a fresh assessment of the projects' current features.

## Product and behavior references supplied by Wayne

| Resource | Link | What it contributed to bears |
| --- | --- | --- |
| OpenMausBot | https://github.com/milind-soni/openmausbot | One of the original three comparisons: named teammates with persistent memory, routines, connected tools and mentions. Helped identify what Threadbear already provided and what needed a small addition. |
| OpenBot, CopilotKit | https://github.com/CopilotKit/OpenBot | Original comparison for teammate agents, approval boundaries and connected tools. Threadbear kept its existing runner, permissions and Connectors rather than adopting another agent platform. |
| Rakazo | https://github.com/elie222/rakazo | Original comparison for persistent teammates, group interactions and shareable agent output. Bears reuse Threadbear threads and existing output surfaces; a separate Pages system was outside this plan. |
| Open Bot Avatar | https://github.com/DominikScholz/open-bot-avatar | JSON-defined avatars rendered as SVG, adapted into bear-shaped presets. The build pinned `open-bot-avatar` 0.2.1 after checking its license and runtime dependencies. |
| Nuphos | https://github.com/nuphos/nuphos | Reinforced named agents running on existing Claude Code/Codex logins, cron/webhook triggers and a shared skills setup. Informed shared memory and the daily tidy. Team rooms, cloud dashboards and chat-service integrations stayed outside scope. |
| Agentbox | https://github.com/savannahfeder/agentbox | Informed durable state outside disposable sessions, one unattended job per bear and preventing agents from widening their own access. Access settings live in runner-owned state. Its additional queue controls and board UI were left out. |
| Cursor: Projects is now multiplayer | https://cursor.com/blog/projects-multiplayer | Informed shared project context, carrying project membership through mentions and checking project state before resuming work. We reused Threadbear project goals, state and notes instead of introducing another store. |
| Instinct teardown by nizzyabi | https://x.com/nizzyabi/status/2120194267066863738 | Informed concise responses, personality, memory maintenance and forgetting, batching and bounded proactive behavior. Wayne supplied the saved Markdown teardown; this is the source URL recorded in that file. |
| Anthropic: Building always-on agents | https://claude.com/resources/articles/building-always-on-agents | Led to the five follow-up improvements: one unattended worker, a record of prior reports and unfinished tasks, quieter memory tidies, explicit quiet empty runs, and late tool-card answers that resume work. |

The Instinct reference was supplied as `~/Downloads/instinct-teardown-nizzyabi.md`. Its saved copy was the material read during planning; the X URL above is retained for provenance.

## Technical references supplied by Wayne

| Resource | Link | How we used it |
| --- | --- | --- |
| Threadbear documentation | https://threadbear.mikecann.app/docs | Existing thread, schedule, approval, project, skill and patch behavior that bears should preserve and reuse. |
| Threadbear upstream repository | https://github.com/mikecann/threadbear | Source of the runner and protocol conventions. The implementation stayed within ordinary threads and experiments, with later UI work tracked for GPUI. |
| plugbear | https://github.com/mikecann/plugbear | Existing connector gateway and tool-scoping reference. Bears inherit Threadbear's skills and connector setup; a separate integrations store was not added. |
| Convex documentation | https://docs.convex.dev/home | Backend patterns for webhook records, delivery, scheduling and owner-scoped operations. |
| Convex documentation index for agents | https://docs.convex.dev/llms.txt | Entry point for finding the relevant Convex documentation during planning and implementation. |
| Convex AI Gateway | https://docs.convex.dev/ai-gateway/overview | Evaluated and left out of the bears architecture. Bears use the existing official Claude Code/Codex CLIs and the user's logins. |

## Supporting references found during the work

These were recorded in the research, but are not attributed to Wayne as pasted links.

| Resource | Link | How we used it |
| --- | --- | --- |
| Convex HTTP actions | https://docs.convex.dev/functions/http-actions | Reference for the webhook HTTP endpoint and request handling. |
| Convex rate limiter repository | https://github.com/get-convex/rate-limiter | Read while planning webhook limits; the component was deliberately left out. |
| Convex rate limiter component listing | https://www.convex.dev/components/rate-limiter | Supporting discovery link for the same evaluated component. |

## Related implementation documents

- [Bears and webhooks](bears-and-webhooks.md): the original build plan.
- [Always-on bears](bears-always-on.md): the five follow-up improvements, review fixes and verification status.
- [Bears for upstream](bears-for-mike.md): the upstream handoff and scope.
- [Bears in GPUI](bears-gpui.md): desktop UI work and behavior to carry forward.
- [User guide](../docs/guide.md), [protocol](../docs/protocol.md) and [experiments](../docs/experiments.md): how the features fit Threadbear.
- [Wayne's fork](https://github.com/waynesutton/threadbear) and [upstream PR #66](https://github.com/mikecann/threadbear/pull/66): implementation destinations, not external inspiration sources.

## Provenance

Compiled from Wayne's messages in the original bears planning thread (`bed8698f`) and build thread (`327a5a9e`), the original bears plan (`threadbear-project-this-thread-flickering-globe.md`), the always-on follow-up plan (`what-can-we-learn-purrfect-feigenbaum.md`), and the saved Instinct teardown. The original three repository links were verified against the first user message in the planning thread. The later references were checked against the user messages and the plans' source lists.
