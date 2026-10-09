const { test, expect } = require("./fixtures.js");
const { randomUUID } = require("node:crypto");

test.beforeEach(async ({ appPage: page }) => {
  await page.evaluate(() => localStorage.setItem("sarmg.admin.language", "zh-CN"));
});

async function expectSharedNavigation(page) {
  const links = page.locator(".xcss-header-navigation a");
  await expect(links).toHaveCount(4);
  expect(await links.evaluateAll(nodes => nodes.map(node => {
    const url = new URL(node.href);
    return url.pathname + url.hash;
  }))).toEqual([
    "/",
    "/__xczs__/tags#files",
    "/__xczs__/tags#tags",
    "/__xczs__/tags#status",
  ]);
}

test("文件与标签页面使用一致的顶部和内容间距", async ({ appPage: page }) => {
  await page.evaluate(() => document.fonts.ready);
  const fileHeaderStyle = await page.locator(".xcss-page-header").evaluate(element => {
    const style = getComputedStyle(element);
    return { paddingTop: style.paddingTop, gap: style.gap, position: style.position };
  });
  expect(await page.locator(".xcss-page-header").evaluate(element => getComputedStyle(element).paddingBottom)).toBe("0px");
  const filePanelStyle = await page.locator(".index-page").evaluate(element => {
    const style = getComputedStyle(element);
    return { padding: style.paddingTop, radius: style.borderTopLeftRadius, border: style.borderTopWidth };
  });
  for (const width of [1838, 390, 320]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/__xczs__/tags#files");
    await expect(page.getByRole("heading", { name: "文件浏览" })).toBeVisible();
    await expect(page.locator('section[aria-label="文件列表"]')).toHaveAttribute("aria-busy", "false");
    await expect(page.getByRole("alert")).toHaveCount(0);
    await page.evaluate(() => document.fonts.ready);
    await expect(page.getByText("字体许可：", { exact: false })).toHaveCount(0);
    const layout = await page.evaluate(() => {
      const header = document.querySelector(".xcss-page-header").getBoundingClientRect();
      const main = document.querySelector(".xcss-shell-main").getBoundingClientRect();
      const heading = document.querySelector(".library-heading").getBoundingClientRect();
      const panel = document.querySelector(".library-panel").getBoundingClientRect();
      const style = getComputedStyle(document.querySelector(".library-panel"));
      const headerStyle = getComputedStyle(document.querySelector(".xcss-page-header"));
      return {
        headingTop: heading.top - header.bottom,
        panelGap: panel.top - heading.bottom,
        panelInset: panel.left - main.left,
        panelWidth: panel.width,
        overflow: document.documentElement.scrollWidth > innerWidth,
        headerStyle: { paddingTop: headerStyle.paddingTop, gap: headerStyle.gap, position: headerStyle.position },
        headerBottomPadding: headerStyle.paddingBottom,
        panelStyle: { padding: style.paddingTop, radius: style.borderTopLeftRadius, border: style.borderTopWidth },
      };
    });
    expect(Math.abs(layout.headingTop)).toBeLessThan(2);
    expect(Math.abs(layout.panelGap - 16)).toBeLessThan(2);
    expect(Math.abs(layout.panelInset - 16)).toBeLessThan(2);
    expect(layout.overflow).toBe(false);
    expect(layout.headerStyle).toEqual(fileHeaderStyle);
    expect(layout.headerBottomPadding).toBe("6px");
    expect(layout.panelStyle).toEqual(filePanelStyle);
    expect(Math.abs(layout.panelWidth - (width - 32))).toBeLessThan(2);
  }
});

test("标签页面通过 Xczs 会话扫描、筛选并批量打标签", async ({ appPage: page }) => {
  const tagName = `回归标签-${randomUUID().slice(0, 8)}`;
  await expectSharedNavigation(page);
  await page.locator('.xcss-header-navigation a[href="/__xczs__/tags#status"]').click();
  await expectSharedNavigation(page);
  await expect(page.locator('.xcss-header-navigation a[aria-current="page"]')).toHaveAttribute("href", "#status");
  const [scan] = await Promise.all([
    page.waitForResponse(response => response.request().method() === "POST" && new URL(response.url()).pathname === "/api/v1/file-tags/scan"),
    page.getByRole("button", { name: "重新扫描" }).click(),
  ]);
  expect(scan.status()).toBe(200);
  expect((await scan.json()).scanned).toBeGreaterThan(0);
  await expect(page.getByText("扫描完成", { exact: false })).toBeVisible();

  await page.locator('.xcss-header-navigation a[href="#tags"]').click();
  await page.getByRole("textbox", { name: "标签名称" }).fill(tagName);
  const [created] = await Promise.all([
    page.waitForResponse(response => response.request().method() === "POST" && new URL(response.url()).pathname === "/api/v1/file-tags/tags"),
    page.getByRole("button", { name: "创建标签" }).click(),
  ]);
  expect(created.status()).toBe(201);
  const tagId = (await created.json()).id;
  expect(Number.isSafeInteger(tagId) && tagId > 0).toBe(true);
  expect(created.request().postDataJSON()).toEqual({ name: tagName, color: "#3b82f6" });
  await expect(page.getByRole("table", { name: "标签列表" })).toContainText(tagName);

  await page.locator('.xcss-header-navigation a[href="#files"]').click();
  await expect(page.getByRole("heading", { name: "文件浏览" })).toBeVisible();
  const fileList = page.locator('section[aria-label="文件列表"]');
  await expect(fileList).toHaveAttribute("aria-busy", "false");
  await page.getByRole("searchbox", { name: "文件名" }).fill("download-me.txt");
  await page.getByLabel("范围").selectOption("all");
  const filteredFiles = response => {
    const url = new URL(response.url());
    return response.request().method() === "GET" && url.pathname === "/api/v1/file-tags/files" && url.searchParams.get("search") === "download-me.txt" && url.searchParams.get("scope") === "all";
  };
  const [filtered] = await Promise.all([
    page.waitForResponse(filteredFiles),
    page.getByRole("button", { name: "筛选", exact: true }).click(),
  ]);
  expect(filtered.status()).toBe(200);
  const file = (await filtered.json()).files.find(file => file.path === "download-me.txt");
  expect(file).toBeDefined();
  await expect(fileList).toHaveAttribute("aria-busy", "false");
  const row = page.getByRole("row").filter({ has: page.getByRole("checkbox", { name: "选择 download-me.txt", exact: true }) });
  await expect(row).toBeVisible();
  await row.getByRole("checkbox").check();
  await page.getByLabel("批量操作的标签").selectOption({ label: tagName });
  const [saved, refreshed] = await Promise.all([
    page.waitForResponse(response => response.request().method() === "POST" && new URL(response.url()).pathname === "/api/v1/file-tags/file-tags"),
    page.waitForResponse(filteredFiles),
    page.getByRole("button", { name: "添加标签" }).click(),
  ]);
  expect(saved.status()).toBe(204);
  expect(saved.request().postDataJSON()).toEqual({ file_ids: [file.id], tag_ids: [tagId], action: "add" });
  expect(refreshed.status()).toBe(200);
  expect((await refreshed.json()).files.find(value => value.id === file.id).tag_ids).toContain(tagId);
  await expect(fileList).toHaveAttribute("aria-busy", "false");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(row).toContainText(tagName);
  await expect(row.getByRole("link", { name: "下载" })).toHaveAttribute("href", "/download-me.txt");
  await page.locator('.xcss-header-navigation a[href="/"]').click();
  await expect(page.locator(".index-page")).toBeVisible();
  await expectSharedNavigation(page);
});
