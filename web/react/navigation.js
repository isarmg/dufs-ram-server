import { t } from "@xcss/admin-ui/i18n";

const sections = [
  { id: "files", zh: "文件", en: "Files", href: "/" },
  { id: "tag-files", zh: "标签浏览", en: "Browse tags", href: "/__xczs__/tags#files", localHref: "#files" },
  { id: "tag-management", zh: "标签管理", en: "Manage tags", href: "/__xczs__/tags#tags", localHref: "#tags" },
  { id: "tag-status", zh: "服务状态", en: "Service status", href: "/__xczs__/tags#status", localHref: "#status" },
];

export function navigationEntries(onTagsPage = false) {
  return sections.map(section => ({
    id: section.id,
    label: t(section.zh, section.en),
    href: onTagsPage ? section.localHref ?? section.href : section.href,
  }));
}
