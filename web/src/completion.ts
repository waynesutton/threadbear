// These are the agent's task reports, never an assertion that runner checks passed.
export type Completion = { tasks: { id: string; title: string; status: 'completed' | 'pending' | 'blocked' }[] };

export function completionHeading(completion: Completion | undefined): string | undefined {
  if (!completion?.tasks.length) return undefined;
  const total = completion.tasks.length;
  const completed = completion.tasks.filter((task) => task.status === 'completed').length;
  if (total === 1 && completed === 1) return '1 task completed';
  return completed === total ? `All ${total} tasks completed` : `${completed} of ${total} tasks completed`;
}
