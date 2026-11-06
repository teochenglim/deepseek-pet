// AC0.4.x behaviour of the real SpriteAnimator state machine (no DOM, frames stubbed).
// Rust wrappers in src-tauri/tests/ac_0_4_interaction.rs run each case by name.
import test from "node:test";
import assert from "node:assert/strict";
import { SpriteAnimator } from "../../src/animator.js";

const ALL = [
  "idle",
  "running",
  "running-left",
  "running-right",
  "failed",
  "waiting",
  "review",
  "waving",
  "jumping",
];

function makeAnimator(start = "idle") {
  const a = new SpriteAnimator();
  for (const s of ALL) a.frames[s] = [1, 2, 3, 4];
  a.ensureAlphaMasksForStates = async () => {};
  a.pruneAlphaMasks = () => {};
  a.updatePlaybackRate = () => {};
  a.currentState = start;
  return a;
}
function tickOneCycle(a) {
  // holds default to 1 tick per frame, so frames.length ticks finish a one-shot
  for (let i = 0; i < 4; i++) a.tick();
}

test("ac_0_4_1_drag_returns_state", () => {
  const a = makeAnimator("running");
  a.handleDrag(4);
  assert.equal(a.currentState, "running-right");
  a.handleDrag(-4);
  assert.equal(a.currentState, "running-left");
  a.handleDrag(0); // release
  assert.equal(a.currentState, "running", "must return to the agent state");
});

test("ac_0_4_1_drag_direction_picks_row", () => {
  const a = makeAnimator("idle");
  a.handleDrag(2);
  assert.equal(a.currentState, "running-right");
  a.handleDrag(0);
  a.handleDrag(-2);
  assert.equal(a.currentState, "running-left");
  a.handleDrag(0);
  assert.equal(a.currentState, "idle");
});

test("ac_0_4_2_click_wave_returns_to_running", () => {
  const a = makeAnimator("running");
  a.triggerOneShot("waving");
  assert.equal(a.currentState, "waving");
  tickOneCycle(a);
  assert.equal(a.currentState, "running", "the wave must not end the working state");
});

test("ac_0_4_2_click_wave_follows_state_change_mid_wave", () => {
  const a = makeAnimator("running");
  a.triggerOneShot("waving");
  a.transitionTo("waiting"); // permission request arrives while waving
  tickOneCycle(a);
  assert.equal(a.currentState, "waiting");
});

test("ac_0_4_2_hover_jump_only_when_idle", () => {
  const busy = makeAnimator("running");
  assert.equal(busy.triggerHoverJump(2), false);
  assert.equal(busy.currentState, "running");
  const idle = makeAnimator("idle");
  assert.equal(idle.triggerHoverJump(2), true);
  assert.equal(idle.currentState, "jumping");
});
