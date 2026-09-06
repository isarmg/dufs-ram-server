import { t } from "../../dist/platform.js";
import { createElement, createIcon } from "../shared/dom.js";

/**
 * @typedef {{
 *   row: HTMLTableRowElement,
 *   statusCell: HTMLTableCellElement,
 *   speedNode: HTMLSpanElement,
 *   progressNode: HTMLSpanElement,
 *   liveNode: HTMLSpanElement,
 *   cancelButton: HTMLButtonElement,
 * }} UploadView
 */

/**
 * @param {number} index
 * @param {string} name
 * @param {string} url
 * @param {() => void} onCancel
 * @returns {UploadView}
 */
export function createUploadView(index, name, url, onCancel) {
  const row = /** @type {HTMLTableRowElement} */ (createElement("tr", {
    className: "uploader",
    attributes: { id: `upload${index}` },
  }));
  const iconCell = createElement("td", { className: "path cell-icon" });
  iconCell.append(createIcon("file"));
  const nameCell = createElement("td", { className: "path cell-name" });
  nameCell.append(createElement("a", {
    text: name,
    attributes: { href: url },
  }));
  const statusCell = /** @type {HTMLTableCellElement} */ (createElement("td", {
    className: "cell-status upload-status",
    attributes: {
      id: `uploadStatus${index}`,
      "aria-label": t("{0}：等待上传", "{0}: waiting to upload", [name]),
    },
  }));
  const speedNode = /** @type {HTMLSpanElement} */ (createElement("span", {
    className: "upload-speed",
    attributes: { "aria-hidden": "true" },
  }));
  const progressNode = /** @type {HTMLSpanElement} */ (createElement("span", {
    className: "upload-progress",
    attributes: { "aria-hidden": "true" },
  }));
  const liveNode = /** @type {HTMLSpanElement} */ (createElement("span", {
    className: "visually-hidden",
    text: t("{0}：等待上传", "{0}: waiting to upload", [name]),
    attributes: { role: "status", "aria-live": "polite" },
  }));
  const cancelButton = /** @type {HTMLButtonElement} */ (createElement("button", {
    className: "upload-cancel",
    text: t("取消", "Cancel"),
    attributes: {
      type: "button",
      "aria-label": t("取消上传 {0}", "Cancel upload {0}", [name]),
    },
  }));
  cancelButton.addEventListener("click", onCancel);
  row.append(iconCell, nameCell, statusCell);
  const view = Object.freeze({
    row,
    statusCell,
    speedNode,
    progressNode,
    liveNode,
    cancelButton,
  });
  renderWaiting(view, name);
  return view;
}

/** @param {UploadView} view @param {string} name @param {boolean} [retry] */
export function renderWaiting(view, name, retry = false) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  setCancelMode(
    view,
    t("取消", "Cancel"),
    t("取消{0} {1}", "Cancel {0} {1}", [retry ? t("排队重试", "queued retry") : t("排队上传", "queued upload"), name]),
  );
  const message = retry ? t("等待重试", "Waiting to retry") : t("等待中", "Waiting");
  view.statusCell.replaceChildren(
    createElement("span", { text: message }),
    view.cancelButton,
    view.liveNode,
  );
  announce(view, `${name}: ${message.toLowerCase()}`);
  restoreReplacedStatusFocus(view, restoreFocus, view.cancelButton);
}

/**
 * @param {UploadView} view
 * @param {string} name
 * @param {string} speedText
 * @param {string} progressText
 * @param {boolean} announceNow
 */
export function renderProgress(view, name, speedText, progressText, announceNow) {
  setCancelMode(view, t("取消", "Cancel"), t("取消上传 {0}", "Cancel upload {0}", [name]));
  if (
    !view.statusCell.contains(view.speedNode) ||
    !view.statusCell.contains(view.progressNode) ||
    !view.statusCell.contains(view.cancelButton) ||
    !view.statusCell.contains(view.liveNode)
  ) {
    const restoreFocus = view.statusCell.contains(document.activeElement);
    // A retry after an overwrite decision re-enters progress rendering from a
    // view that intentionally has no cancel button. Rebuild the owned status
    // nodes atomically instead of using a possibly detached node as the
    // insertBefore reference.
    view.statusCell.replaceChildren(
      view.speedNode,
      view.progressNode,
      view.cancelButton,
      view.liveNode,
    );
    restoreReplacedStatusFocus(view, restoreFocus, view.cancelButton);
  }
  view.speedNode.textContent = speedText;
  view.progressNode.textContent = progressText;
  if (announceNow) {
    announce(view, t("{0}：已上传 {1}，速度 {2}", "{0}: {1} uploaded at {2}", [name, progressText, speedText]));
  }
}

/** @param {UploadView} view @param {string} name */
export function renderCheckpoint(view, name) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  setCancelMode(view, t("取消", "Cancel"), t("取消 {0} 的续传状态核对", "Cancel resume status check for {0}", [name]));
  view.statusCell.replaceChildren(
    createElement("span", { text: t("正在核对续传状态…", "Checking resume status…") }),
    view.cancelButton,
    view.liveNode,
  );
  announce(view, t("{0}：正在核对续传状态", "{0}: checking resume status", [name]));
  restoreReplacedStatusFocus(view, restoreFocus, view.cancelButton);
}

/** @param {UploadView} view @param {string} name */
export function renderCleanup(view, name) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  view.statusCell.replaceChildren(
    createElement("span", { text: t("正在清理暂存上传…", "Cleaning up staged upload…") }),
    view.liveNode,
  );
  announce(view, t("{0}：正在清理暂存上传", "{0}: cleaning up staged upload", [name]));
  restoreReplacedStatusFocus(view, restoreFocus);
}

/** @param {UploadView} view @param {string} name */
export function renderSubmitting(view, name) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  setCancelMode(view, t("停止等待", "Stop waiting"), t("停止等待上传 {0}", "Stop waiting for upload {0}", [name]));
  view.statusCell.replaceChildren(
    createElement("span", { text: t("正在提交…", "Submitting…") }),
    view.cancelButton,
    view.liveNode,
  );
  announce(view, t("{0}：上传数据已发送，等待服务器确认", "{0}: upload data sent; waiting for server confirmation", [name]));
  restoreReplacedStatusFocus(view, restoreFocus, view.cancelButton);
}

/** @param {UploadView} view @param {string} name */
export function renderWaitingForOverwrite(view, name) {
  view.statusCell.replaceChildren(
    createElement("span", { text: t("等待覆盖决定…", "Waiting for overwrite decision…") }),
    view.liveNode,
  );
  announce(view, t("{0}：等待覆盖决定", "{0}: waiting for overwrite decision", [name]));
}

/** @param {UploadView} view @param {string} name */
export function renderComplete(view, name) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  view.statusCell.replaceChildren(
    createElement("span", {
      text: "✓",
      attributes: { "aria-hidden": "true" },
    }),
    view.liveNode,
  );
  announce(view, t("{0}：上传完成", "{0}: upload complete", [name]));
  restoreReplacedStatusFocus(view, restoreFocus);
}

/**
 * @param {UploadView} view
 * @param {string} name
 * @param {string} reason
 * @param {HTMLButtonElement | null} [retryButton]
 */
export function renderFailure(view, name, reason, retryButton = null) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  view.statusCell.replaceChildren(createElement("span", {
    className: "upload-failure",
    text: `✗ ${reason}`,
    attributes: { title: reason },
  }));
  if (retryButton) view.statusCell.append(retryButton);
  view.statusCell.append(view.liveNode);
  announce(view, `${name}: ${reason}`);
  restoreReplacedStatusFocus(view, restoreFocus, retryButton);
}

/**
 * @param {UploadView} view
 * @param {string} name
 * @param {string} reason
 * @param {HTMLButtonElement | null} [recoveryButton]
 */
export function renderUnknown(view, name, reason, recoveryButton = null) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  view.statusCell.replaceChildren(
    createElement("span", {
      className: "upload-failure upload-unknown",
      text: `? ${reason}`,
      attributes: { title: reason },
    }),
  );
  if (recoveryButton) view.statusCell.append(recoveryButton);
  view.statusCell.append(view.liveNode);
  announce(view, t("{0}：上传结果不确定；{1}", "{0}: upload result unknown; {1}", [name, reason]));
  restoreReplacedStatusFocus(view, restoreFocus, recoveryButton);
}

/** @param {UploadView} view @param {string} name */
export function renderCancelled(view, name) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  view.statusCell.replaceChildren(
    createElement("span", { text: t("已取消", "Cancelled") }),
    view.liveNode,
  );
  announce(view, t("{0}：上传已取消", "{0}: upload cancelled", [name]));
  restoreReplacedStatusFocus(view, restoreFocus);
}

/** @param {UploadView} view @param {string} name @param {string} reason */
export function renderSkipped(view, name, reason) {
  const restoreFocus = view.statusCell.contains(document.activeElement);
  view.statusCell.replaceChildren(
    createElement("span", { text: t("已跳过（{0}）", "Skipped ({0})", [reason]) }),
    view.liveNode,
  );
  announce(view, t("{0}：已跳过，原因：{1}", "{0}: skipped because the {1}", [name, reason]));
  restoreReplacedStatusFocus(view, restoreFocus);
}

/**
 * Keep keyboard focus in a useful place when a status transition removes the
 * currently focused Cancel/Stop-waiting control.
 *
 * @param {UploadView} view
 * @param {boolean} restoreFocus
 * @param {HTMLButtonElement | null} [preferredFocus]
 */
function restoreReplacedStatusFocus(
  view,
  restoreFocus,
  preferredFocus = null,
) {
  if (!restoreFocus) return;
  /** @type {HTMLElement | null} */
  let target = preferredFocus;
  if (!target?.isConnected || target.hasAttribute("disabled")) {
    const nameLink = view.row.querySelector(".cell-name a");
    target = nameLink instanceof HTMLElement ? nameLink : null;
  }
  if (target) {
    target.focus({ preventScroll: true });
    return;
  }
  view.statusCell.tabIndex = -1;
  view.statusCell.focus({ preventScroll: true });
}

/** @param {UploadView} view @param {string} message */
function announce(view, message) {
  view.statusCell.setAttribute("aria-label", message);
  view.liveNode.textContent = message;
}

/**
 * @param {UploadView} view
 * @param {string} text
 * @param {string} accessibleName
 */
function setCancelMode(view, text, accessibleName) {
  view.cancelButton.textContent = text;
  view.cancelButton.setAttribute("aria-label", accessibleName);
}
