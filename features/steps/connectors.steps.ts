// The Connectors panel (features/connectors.feature). Its lines are what it
// draws: the gateway's machine first, then "<service>: <state>" with its
// button in brackets, and its message under it, indented.
import { Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import type { ThreadbearWorld } from '../support/world.ts';

Then("the Connectors panel's heading is {string}", async function (this: ThreadbearWorld, heading: string) {
  await this.until(`the Connectors panel headed "${heading}"`, (d) => d.connectors?.lines[0] === heading);
});

When('no gateway is set up and setup is available', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mock', method: 'connectors', params: { state: 'off', canSetUp: true } });
});

When('the connector catalog offers Slack', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mock', method: 'connectors', params: { catalog: [{ id: 'slack', title: 'Slack', blurb: 'Search your workspace.' }] } });
});

When('the connector catalog offers a local server', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mock', method: 'connectors', params: { local: true } });
});

When('the outside-agent list temporarily fails', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'mock', method: 'connectors', params: { agentsListError: true } });
});

Then('the outside-agent refresh leaves no error banner', async function (this: ThreadbearWorld) {
  assert.equal((await this.app.dump()).error, null);
});

Then('the connector setup thread is open', async function (this: ThreadbearWorld) {
  await this.until('the returned setup thread is open', (d) => d.selection?.id === 'demo_upload_resume');
});

Then('the Connectors panel never displays the new secret', async function (this: ThreadbearWorld) {
  const d = await this.app.dump();
  assert(!JSON.stringify(d).includes('f00d'), 'the mock key must not appear in a dump');
  await this.app.do({ cmd: 'click', tipPrefix: 'Copy "Bearer"', connectors: true });
  const copied = await this.app.do({ cmd: 'clipboard' });
  assert(String(copied).includes('Bearer tba_' + 'f00d'.repeat(16)), 'copy includes the full bearer header');
});

Then('the Connectors rail has no sign-in alert', async function (this: ThreadbearWorld) {
  await this.until('no hidden service alert', (d) => d.rails?.activity.find((b) => b.title === 'Connectors')?.dot === false);
});

When('I restore the hidden Sentry connector', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'click', tipPrefix: 'Bring back a connector you hid', connectors: true });
  await this.app.do({ cmd: 'click', title: 'Show Sentry' });
});

When('I close and reopen the Connectors panel', async function (this: ThreadbearWorld) {
  await this.app.do({ cmd: 'connectors', show: false });
  await this.until('the Connectors panel closes', (d) => d.connectors?.shown === false);
  await this.app.do({ cmd: 'connectors', show: true });
  await this.until('the Connectors panel reopens', (d) => d.connectors?.shown === true);
});

Then("the Connectors panel doesn't list {string}", async function (this: ThreadbearWorld, line: string) {
  await this.until(`no "${line}" in the Connectors panel`, (d) => d.connectors !== undefined && !d.connectors.lines.includes(line));
});
