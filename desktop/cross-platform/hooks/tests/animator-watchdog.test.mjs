// AC0.8.8: a working animation never runs forever; it ends when the agent goes quiet.
import test from "node:test";
import assert from "node:assert/strict";
import { SpriteAnimator } from "../../src/animator.js";

function makeAnimator() {
  const a = new SpriteAnimator();
  for (const s of ["idle", "running", "review", "waiting", "waving", "jumping"])
    a.frames[s] = [1, 2, 3, 4];
  a.ensureAlphaMasksForStates = async () => {};
  a.pruneAlphaMasks = () => {};
  a.updatePlaybackRate = () => {};
  a.workingTimeoutMs = 20;
  return a;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

test("ac_0_8_8_working_state_falls_back_to_idle_when_agent_goes_quiet", async () => {
  const a = makeAnimator();
  let notified = 0;
  a.onWorkingTimeout = () => notified++;
  a.transitionTo("running");
  assert.equal(a.currentState, "running");
  await sleep(60);
  assert.equal(a.currentState, "idle");
  assert.equal(notified, 1);
});

test("ac_0_8_8_new_event_keeps_working_state_alive", async () => {
  const a = makeAnimator();
  a.transitionTo("review");
  await sleep(12);
  a.transitionTo("running"); // next hook event re-arms
  await sleep(12);
  assert.equal(a.currentState, "running", "still inside the new window");
  await sleep(40);
  assert.equal(a.currentState, "idle");
});

test("ac_0_8_8_stop_and_waiting_cancel_the_watchdog", async () => {
  const a = makeAnimator();
  a.transitionTo("running");
  a.triggerOneShot("jumping"); // Stop
  a.transitionTo("waiting"); // approval pending must not be timed out
  await sleep(60);
  assert.equal(a.currentState, "waiting");
});
