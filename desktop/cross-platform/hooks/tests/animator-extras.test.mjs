// AC0.6.3: manually triggered clips from the Play animation menu, on the real SpriteAnimator.
// Rust wrappers in src-tauri/tests/ac_0_6_extras.rs run each case by name.
import test from "node:test";
import assert from "node:assert/strict";
import { SpriteAnimator } from "../../src/animator.js";

function makeAnimator(start = "idle") {
  const a = new SpriteAnimator();
  for (const s of ["idle", "running", "waiting", "waving", "jumping"]) a.frames[s] = [1, 2, 3, 4];
  a.loaded = [];
  a.loadImage = async (path) => {
    a.loaded.push(path);
    return { src: path, dataset: { objectUrl: "blob:test" } };
  };
  a.ensureAlphaMasksForStates = async () => {};
  a.pruneAlphaMasks = () => {};
  a.updatePlaybackRate = () => {};
  a.currentState = start;
  return a;
}
const CLIP = { id: "c042", frames: 5, fps: 8 };

test("ac_0_6_3_extra_loads_lazily_and_plays_once", async () => {
  const a = makeAnimator("idle");
  assert.equal(
    Object.keys(a.frames).some((s) => s.startsWith("extra:")),
    false,
    "nothing extra is loaded at startup",
  );
  assert.equal(await a.playExtra(CLIP, "/frames"), true);
  assert.equal(a.currentState, "extra:c042");
  assert.equal(a.loaded.length, 5);
  assert.equal(a.loaded[0], "/frames/extras/c042/00.png");
  for (let i = 0; i < 5; i++) a.tick(); // one pass
  assert.equal(a.currentState, "idle", "returns to the previous state after one pass");
});

test("ac_0_6_3_extra_frames_freed_after_play", async () => {
  const a = makeAnimator("running");
  await a.playExtra(CLIP, "/frames");
  for (let i = 0; i < 5; i++) a.tick();
  assert.equal(a.currentState, "running", "a working fish keeps working");
  assert.equal(a.frames["extra:c042"], undefined, "decoded frames are released");
  assert.equal(a.framePaths["extra:c042"], undefined);
  assert.equal(a.extraFps.has("extra:c042"), false);
});

test("ac_0_6_3_state_change_during_extra_is_kept", async () => {
  const a = makeAnimator("running");
  await a.playExtra(CLIP, "/frames");
  a.transitionTo("waiting"); // permission request arrives mid-clip
  assert.equal(a.currentState, "waiting");
  assert.equal(a.frames["extra:c042"], undefined, "clip is dropped when it is cut short");
});

test("ac_0_6_3_missing_clip_files_do_not_break_the_pet", async () => {
  const a = makeAnimator("idle");
  a.loadImage = async () => null;
  assert.equal(await a.playExtra(CLIP, "/frames"), false);
  assert.equal(a.currentState, "idle");
});

test("ac_0_6_3_extra_uses_its_own_fps", async () => {
  const a = makeAnimator("idle");
  await a.playExtra({ id: "c001", frames: 3, fps: 12 }, "/frames");
  assert.equal(a.getTargetFps(), 12);
});
