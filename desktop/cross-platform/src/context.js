// Show Claude context: a floating window of its own. Rust passes the text and how long it stays
// (window.__NOTE, set before this script runs) and closes the window itself when the time is up.
const note = window.__NOTE || { text: "", ms: 20000 };
const el = document.getElementById("note");
el.textContent = note.text;

// fade over the last two seconds
setTimeout(() => el.classList.add("fading"), Math.max(0, note.ms - 2000));

// click to close sooner
el.addEventListener("click", () => {
  const tauri = window.__TAURI__;
  if (tauri && tauri.window) tauri.window.getCurrentWindow().close();
});
