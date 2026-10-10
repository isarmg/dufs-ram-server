import { t } from "@xcss/web/admin-ui/i18n";

export type WorkspaceView = "files" | "tags" | "status" | "account";

const sections = [
  { id: "files", zh: "文件", en: "Files", href: "#files" },
  {
    id: "tags",
    zh: "标签管理",
    en: "Manage tags",
    href: "#tags",
  },
  {
    id: "status",
    zh: "服务状态",
    en: "Service status",
    href: "#status",
  },
] as const;

export function workspaceView(hash = window.location.hash): WorkspaceView {
  const view = hash.slice(1);
  return view === "tags" || view === "status" || view === "account"
    ? view
    : "files";
}

/** Keep existing tag bookmarks while using the directory application entry. */
export function normalizeWorkspaceLocation() {
  if (window.location.pathname !== "/__xczs__/tags") {
    if (
      window.location.hash === "#tag-files" ||
      window.location.hash === "#files/tags"
    ) {
      window.history.replaceState(window.history.state, "", "#files");
    }
    return;
  }
  const hash = window.location.hash;
  const view =
    hash === "#tags" || hash === "#status" || hash === "#account"
      ? hash
      : "#files";
  window.history.replaceState(
    window.history.state,
    "",
    `/${window.location.search}${view}`,
  );
}

export function navigationEntries() {
  return sections.map((section) => ({
    id: section.id,
    label: t(section.zh, section.en),
    href: section.href,
  }));
}
