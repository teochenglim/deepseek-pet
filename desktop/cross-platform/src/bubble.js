/**
 * 对话气泡——等价于 DialogueBubble.swift
 * 在宠物上方显示文本，有 3 种样式：normal、warning、error。
 */

/** 非持久气泡保持可见多久后自动淡出（毫秒）。 */
const AUTO_HIDE_MS = 3000;

/** 保持可见直到被下一个事件替换的状态——agent 正在工作中、等待用户输入或处于
 * 错误态。其余一切（如 jumping/waving 的庆祝、idle 问候、通知）都会在 AUTO_HIDE_MS
 * 后自动淡出，以免气泡永远停留。 */
const PERSISTENT_STATES = new Set([
  "running",
  "running-left",
  "running-right",
  "waiting",
  "failed",
]);
const VALID_STYLES = new Set(["normal", "warning", "error"]);

export class DialogueBubble {
  constructor(element, badgeElement, config) {
    this.el = element;
    this.badgeEl = badgeElement;
    this.textEl = element.querySelector("#bubble-text");
    this.countEl = element.querySelector("#bubble-count");
    this.config = config;
    this.currentStyle = "normal";
    this.collapsed = false;
    this.hideTimer = null;
    this.latest = {
      text: "",
      sessionCount: 0,
      state: "idle",
      forceAutoHide: false,
      hookEvent: "",
      pendingPermissionCount: 0,
    };
    this.applyConfigStyles();
    this.attachToggleHandlers();
  }

  /** 从 config 应用可配置的对话尺寸/时序。
   *
   * 这些与 .bubble 的 CSS 默认值对应（字体用 pt、尺寸用 px、淡出用 s），因此
   * 自带的配置值能复现内置外观，同时允许 config.json 覆盖它们。此前 Rust →
   * 前端的管线把这些字段放进了 `config`，却从没有任何地方读取它们，所以在
   * config.json 中自定义 font_size / max_width / corner_radius / fade_duration
   * 毫无效果。 */
  applyConfigStyles() {
    const c = this.config || {};
    if (c.dialogue_font_size != null) {
      this.el.style.fontSize = `${c.dialogue_font_size}pt`;
    }
    if (c.dialogue_max_width != null) {
      this.el.style.maxWidth = `${c.dialogue_max_width}px`;
    }
    if (c.dialogue_corner_radius != null) {
      this.el.style.borderRadius = `${c.dialogue_corner_radius}px`;
    }
    if (c.dialogue_fade_duration != null) {
      this.el.style.transition = `opacity ${c.dialogue_fade_duration}s ease`;
    }
  }

  /** 显示气泡，包含文本、会话计数和基于状态的样式。
   *
   * 持久状态（running/waiting/failed——agent 忙碌、等待输入或出错）保持可见，
   * 直到被下一个事件替换。其余所有状态在 AUTO_HIDE_MS 后自动淡出。传入
   * forceAutoHide=true 可覆盖该行为并总是淡出——用于一次性用户反馈（例如
   * 三连击的清除结果），即便其状态通常是持久的（failed），也不应久留。 */
  show(
    text,
    sessionCount = 0,
    state = "idle",
    forceAutoHide = false,
    hookEvent = "",
    pendingPermissionCount = 0,
  ) {
    this.endNote();
    this.latest = {
      text,
      sessionCount,
      state,
      forceAutoHide,
      hookEvent,
      pendingPermissionCount,
    };
    this.render();
  }

  /** 显示多行说明（Show Claude context），保持 `ms` 毫秒后淡出；忽略折叠状态。
   * 说明显示期间窗口会变大（见 main.js），结束时调用 onNoteEnd 让窗口恢复原样。 */
  showNote(text, ms = 15000) {
    this.collapsed = false;
    this.noteActive = false; // show() must not end the note that is starting
    this.show(text, 0, "idle", true);
    this.el.classList.add("note");
    this.noteActive = true;
    this.clearAutoHide();
    this.hideTimer = setTimeout(() => this.hide(), ms);
  }

  /** 说明结束（被新消息替换、淡出或被点击收起）：通知窗口恢复。 */
  endNote() {
    if (!this.noteActive) return;
    this.noteActive = false;
    this.el.classList.remove("note");
    if (this.onNoteEnd) this.onNoteEnd();
  }

  render() {
    const { text, sessionCount, state, forceAutoHide, pendingPermissionCount } = this.latest;
    this.updateBadge(sessionCount, pendingPermissionCount);

    if (this.collapsed || (!text && sessionCount <= 1)) {
      this.hideBubble();
      return;
    }

    // 样式来自 config.dialogue.style_map（waiting → warning，failed → error）。
    // 未映射的状态默认为 "normal"。
    const style = (this.config.style_map && this.config.style_map[state]) || "normal";

    this.applyStyle(style);
    this.textEl.textContent = text;
    this.countEl.textContent = sessionCount > 1 ? `×${sessionCount}` : "";

    this.el.classList.remove("hidden");
    this.el.classList.add("visible");

    if (PERSISTENT_STATES.has(state) && !forceAutoHide) {
      this.clearAutoHide();
    } else {
      this.scheduleAutoHide();
    }
  }

  /** 淡出隐藏气泡 */
  hide() {
    this.latest = {
      text: "",
      sessionCount: 0,
      state: "idle",
      forceAutoHide: false,
      hookEvent: "",
      pendingPermissionCount: 0,
    };
    this.updateBadge(0, 0);
    this.hideBubble();
  }

  hideBubble() {
    this.endNote();
    this.clearAutoHide();
    this.el.classList.remove("visible");
    this.el.classList.add("hidden");
  }

  /** 启动（或重置）自动淡出定时器。每次 show() 都会重置它，因此一连串事件能
   * 让气泡保持存活；只有事件停止后才会淡出。 */
  scheduleAutoHide() {
    this.clearAutoHide();
    this.hideTimer = setTimeout(() => this.hide(), AUTO_HIDE_MS);
  }

  clearAutoHide() {
    if (this.hideTimer) {
      clearTimeout(this.hideTimer);
      this.hideTimer = null;
    }
  }

  applyStyle(style) {
    const nextStyle = VALID_STYLES.has(style) ? style : "normal";
    if (this.currentStyle === nextStyle) return;
    this.el.classList.remove("style-normal", "style-warning", "style-error");
    this.el.classList.add(`style-${nextStyle}`);
    this.currentStyle = nextStyle;
  }

  attachToggleHandlers() {
    const stopPetInteraction = (event) => {
      event.preventDefault();
      event.stopPropagation();
    };

    ["mousedown", "mouseup", "mousemove", "contextmenu"].forEach((eventName) => {
      this.el.addEventListener(eventName, stopPetInteraction);
      this.badgeEl.addEventListener(eventName, stopPetInteraction);
    });
    this.el.addEventListener("click", (event) => {
      stopPetInteraction(event);
      if (this.el.classList.contains("visible")) {
        this.collapsed = true;
        this.render();
      }
    });

    this.badgeEl.addEventListener("click", (event) => {
      stopPetInteraction(event);
      this.collapsed = false;
      this.render();
    });
  }

  updateBadge(sessionCount, pendingPermissionCount) {
    const shouldShow = this.collapsed && sessionCount > 0;
    this.badgeEl.textContent = sessionCount > 9 ? "9+" : String(sessionCount);
    this.badgeEl.classList.toggle("hidden", !shouldShow);
    this.badgeEl.classList.toggle("warning", pendingPermissionCount > 0);
  }
}
