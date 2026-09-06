import { createElement as h, Fragment, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { flushSync } from "react-dom";
import { Button, IconButton, TextField, Table } from "@sarmg/admin-ui";
import { WorkspaceIcon, resolveWorkspaceConfig } from "@sarmg/admin-shell";
import { t, initializeLanguage, languageLabel, switchLanguage, validationMessage } from "@sarmg/admin-ui/i18n";
import { isAdministratorLoginRequest } from "@sarmg/contracts";
import { isAdministratorPassword } from "@sarmg/admin-web";

/** @type {WeakSet<Element>} */
const mounted = new WeakSet();
const workspace = resolveWorkspaceConfig({ layout: "custom" });

/** A single React root owns the authored page. File controllers own only the
 * opaque file/queue/dialog regions after the initial synchronous render.
 * Theme updates are confined to their own component, so React never reconciles
 * an active upload row or inline editor. Navigation remains full-document.
 * @param {HTMLElement} container @param {import("react").ReactNode} page */
function mount(container, page) {
  if (mounted.has(container)) throw new Error("Dufs application is already mounted");
  mounted.add(container);
  initializeLanguage();
  document.documentElement.dataset.sarmgAppearance = workspace.appearance;
  document.documentElement.dataset.sarmgSelection = workspace.selection;
  document.documentElement.style.setProperty("--sarmg-font-ui", workspace.fontFamily);
  document.documentElement.style.setProperty("--sarmg-header-icon-size", workspace.headerIconSize);
  const root = createRoot(container);
  flushSync(() => root.render(page));
}

/** @param {HTMLElement} container */
export function mountFileWorkspace(container) {
  mount(container, h(FileWorkspace));
}

/** @param {HTMLElement} container
 * @param {(username: string, password: string) => Promise<void>} login
 * @param {(error: unknown) => string} errorMessage */
export function mountLoginPage(container, login, errorMessage) {
  const minimum = Number(container.dataset.minBytes);
  const maximum = Number(container.dataset.maxBytes);
  if (!Number.isInteger(minimum) || !Number.isInteger(maximum) || minimum < 1 || maximum < minimum) {
    throw new Error("Invalid administrator password contract");
  }
  mount(container, h(LoginPage, { login, errorMessage, minimum, maximum }));
  document.title = t("登录", "Sign in");
}

function LanguageToggle() {
  return h(IconButton, { "aria-label": languageLabel(), title: languageLabel(), onClick: switchLanguage },
    h("svg", { "aria-hidden": true, width: "1em", height: "1em", viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 1.7 },
      h("path", { d: "M3 5h12M9 3v2M6 5c0 5 4 9 8 11M13 5c0 5-4 9-9 12m10 4 4-10 4 10m-6.5-4h5" })));
}

function ThemeToggle() {
  const [dark, setDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => { document.documentElement.dataset.theme = dark ? "dark" : "light"; }, [dark]);
  const label = dark ? t("切换到浅色模式", "Switch to light mode") : t("切换到深色模式", "Switch to dark mode");
  return h(IconButton, { "aria-label": label, title: label, onClick: () => setDark(value => !value) },
    h(WorkspaceIcon, { name: dark ? "sun" : "moon" }));
}

function Header() {
  return h("header", { className: "head sarmg-page-header" },
    h("div", { className: "sarmg-header-navigation-slot" }, h("div", { className: "sarmg-header-brand-navigation" },
      h("strong", { className: "sarmg-product-identity" }, "Dufs"),
      h("nav", { className: "sarmg-header-navigation", "aria-label": t("主导航", "Main navigation") },
        h("a", { href: "/", "aria-current": "page" }, t("文件", "Files"))))),
    h("div", { className: "toolbox-right sarmg-header-actions", role: "group", "aria-label": t("全局操作", "Global actions") },
      h(IconButton, { className: "new-folder hidden", "aria-label": t("新建文件夹", "New folder"), title: t("新建文件夹", "New folder") }, h(WorkspaceIcon, { name: "create" })),
      h(IconButton, { "aria-label": t("重新载入页面", "Reload page"), title: t("重新载入页面", "Reload page"), onClick: () => window.location.reload() }, h(WorkspaceIcon, { name: "refresh" })),
      h(LanguageToggle), h(ThemeToggle),
      h(IconButton, { className: "logout-btn hidden", "aria-label": t("退出", "Sign out"), title: t("退出", "Sign out") },
        h(WorkspaceIcon, { name: "logout" }), h("span", { className: "user-name", hidden: true }))));
}

/** @param {{ name: "upload" | "folder" | "file" | "search" }} props */
function FileIcon({ name }) {
  const paths = {
    upload: "M12 16V3m-5 5 5-5 5 5M4 15v6h16v-6",
    folder: "M3 7V4h6l3 3h9v14H3V7m9 11v-8m-3 3 3-3 3 3",
    file: "M5 3h9l5 5v13H5V3m9 0v6h5M9 15h6m-3-3v6",
    search: "M17 17l5 5M19 10a9 9 0 1 1-18 0 9 9 0 0 1 18 0",
  };
  return h("svg", { "aria-hidden": true, width: 16, height: 16, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 1.7 }, h("path", { d: paths[name] }));
}

function FileControls() {
  return h("section", { className: "sarmg-content-panel file-toolbar", "aria-label": t("文件操作", "File controls") },
    h("nav", { className: "breadcrumb", "aria-label": t("当前文件夹", "Current folder") }),
    h("div", { className: "toolbox" },
      h(IconButton, { className: "control upload-file hidden", "aria-label": t("上传文件", "Upload files"), title: t("上传文件", "Upload files") }, h(FileIcon, { name: "upload" })),
      h("input", { className: "visually-hidden", type: "file", id: "file", name: "file", "aria-label": t("选择要上传的文件", "Choose files to upload"), tabIndex: -1, multiple: true }),
      h(IconButton, { className: "control upload-folder hidden", "aria-label": t("上传文件夹", "Upload folder"), title: t("上传文件夹", "Upload folder") }, h(FileIcon, { name: "folder" })),
      h("input", { className: "visually-hidden", type: "file", id: "folder", name: "folder", "aria-label": t("选择要上传的文件夹", "Choose a folder to upload"), tabIndex: -1, webkitdirectory: "", multiple: true }),
      h(IconButton, { className: "control new-file hidden", "aria-label": t("新建空文件", "New empty file"), title: t("新建空文件", "New empty file") }, h(FileIcon, { name: "file" }))),
    h("form", { className: "searchbar hidden" },
      h("div", { className: "icon" }, h(FileIcon, { name: "search" })),
      h("label", { className: "visually-hidden", htmlFor: "search" }, t("搜索文件或文件夹", "Search files or folders")),
      h("input", { id: "search", name: "q", type: "text", maxLength: 128, autoComplete: "off", title: t("搜索文件或文件夹", "Search files or folders"), "aria-label": t("搜索文件或文件夹", "Search files or folders") }),
      h("input", { type: "submit", hidden: true })));
}

function FileContent() {
  return h("div", { className: "index-page sarmg-content-panel hidden" },
    h("div", { className: "operation-status hidden", role: "status", "aria-live": "polite" }),
    h("div", { className: "upload-queue-message hidden", role: "alert", "aria-live": "assertive" }),
    h("div", { className: "empty-folder hidden", role: "status", "aria-live": "polite" }),
    h(Table, { className: "uploaders-table hidden", "aria-label": t("上传队列", "Upload queue") },
      h("caption", { className: "upload-history-status hidden", role: "status", "aria-live": "polite" }),
      h("thead", null, h("tr", null,
        h("th", { className: "cell-name", colSpan: 2 }, t("名称", "Name")), h("th", { className: "cell-status" }, t("上传状态", "Upload status"))))),
    h(Table, { className: "paths-table hidden", "aria-label": t("文件列表", "File list") },
      h("colgroup", null, h("col", { className: "file-icon" }), h("col"), h("col", { className: "file-modified" }), h("col", { className: "file-size" }), h("col", { className: "file-actions" })),
      h("thead"), h("tbody")),
    h("div", { className: "list-pagination" },
      h("div", { className: "list-status", role: "status", "aria-live": "polite" }),
      h(Button, { className: "load-more hidden" }, t("加载更多", "Load more"))));
}

function ActionDialog() {
  return h("dialog", { className: "action-dialog sarmg-dialog", "aria-labelledby": "action-dialog-title", "aria-describedby": "action-dialog-message" },
    h("form", { className: "action-dialog-form", method: "dialog" },
      h("h2", { id: "action-dialog-title" }), h("p", { id: "action-dialog-message" }),
      h("div", { className: "action-dialog-input-group" }, h("label", { htmlFor: "action-dialog-input" }), h("input", { id: "action-dialog-input", type: "text", autoComplete: "off" })),
      h("div", { className: "action-dialog-actions" },
        h(Button, { className: "action-dialog-cancel" }, t("取消", "Cancel")),
        h(Button, { className: "action-dialog-alternate", type: "submit", value: "alternate", hidden: true }, t("跳过", "Skip")),
        h(Button, { className: "action-dialog-confirm", type: "submit", value: "confirm" }, t("确认", "Confirm")))));
}

function FileWorkspace() {
  return h(Fragment, null,
    h("a", { className: "sarmg-skip-link", href: "#file-content" }, t("跳转到文件内容", "Skip to files")),
    h(Header),
    h("main", { className: "main sarmg-shell-main", id: "file-content", tabIndex: -1 }, h(FileControls), h(FileContent)),
    h(ActionDialog));
}

/** @param {{ login(username: string, password: string): Promise<void>, errorMessage(error: unknown): string, minimum: number, maximum: number }} props */
function LoginPage({ login, errorMessage, minimum, maximum }) {
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const [failure, setFailure] = useState({ message: "", field: "" });
  /** @param {import("react").FormEvent<HTMLElement>} event */
  async function submit(event) {
    event.preventDefault();
    if (busy.current) return;
    const form = event.currentTarget;
    if (!(form instanceof HTMLFormElement)) return;
    const username = form.elements.namedItem("username");
    const password = form.elements.namedItem("password");
    if (!(username instanceof HTMLInputElement) || !(password instanceof HTMLInputElement)) return;
    for (const input of [username, password]) {
      if (!input.validity.valid) {
        const message = input.validity.valueMissing
          ? input === username ? t("请输入用户名。", "Enter your username.") : t("请输入密码。", "Enter your password.")
          : validationMessage(input);
        setFailure({ message, field: input.name }); input.focus(); return;
      }
    }
    if (!isAdministratorLoginRequest({ username: username.value, password: password.value }) || !isAdministratorPassword(password.value)) {
      password.value = "";
      setFailure({ message: t("请输入有效的管理员用户名和密码。", "Enter a valid administrator username and password."), field: "password" });
      password.focus(); return;
    }
    busy.current = true; setPending(true); setFailure({ message: "", field: "" });
    // Apply the lock immediately, before another browser event can submit.
    username.readOnly = true; password.readOnly = true;
    let failed = false;
    try { await login(username.value, password.value); }
    catch (error) { failed = true; setFailure({ message: errorMessage(error), field: "" }); }
    finally {
      password.value = ""; username.readOnly = false; password.readOnly = false;
      busy.current = false; setPending(false); if (failed) password.focus();
    }
  }
  return h("main", { className: "sarmg-auth-shell login-screen" },
    h("section", { className: "sarmg-auth-card", "aria-label": "Dufs" },
      h("form", { className: "login-card", "aria-label": t("登录 Dufs", "Sign in to Dufs"), noValidate: true, "aria-busy": pending || undefined,
        onSubmit: event => { void submit(event); }, onInvalid: event => event.preventDefault(), onInput: () => setFailure({ message: "", field: "" }) },
      h("h1", null, t("登录", "Sign in")),
      h("label", { className: "sarmg-form-field", htmlFor: "username" }, h("span", null, t("用户名", "Username")),
        h(TextField, { className: "login-input", id: "username", name: "username", type: "text", maxLength: 64, autoComplete: "username", autoCapitalize: "none", spellCheck: false, required: true, autoFocus: true, readOnly: pending,
          "aria-invalid": failure.field === "username" || undefined, "aria-describedby": failure.message ? "login-error" : undefined })),
      h("label", { className: "sarmg-form-field", htmlFor: "password" }, h("span", null, t("密码", "Password")),
        h(TextField, { className: "login-input", id: "password", name: "password", type: "password", ...{ "data-min-bytes": minimum, "data-max-bytes": maximum }, autoComplete: "current-password", required: true, readOnly: pending,
          "aria-invalid": failure.field === "password" || undefined, "aria-describedby": failure.message ? "login-error" : undefined })),
      h("div", { className: `sarmg-error error-row${failure.message ? "" : " hidden"}`, role: "alert" }, h("span", { className: "login-error", id: "login-error" }, failure.message)),
      h(Button, { type: "submit", disabled: pending }, t("登录", "Sign in")),
      h("div", { className: "card-actions" }, h(LanguageToggle)))));
}
