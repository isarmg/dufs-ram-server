const { readFileSync } = require("node:fs");
const AxeBuilder = require("@axe-core/playwright").default;
const { test, expect, pageData, login, selectFiles } = require("./fixtures.js");

test("React Profile 的实际嵌入字体、许可证和恢复会话来自 Foundation", async ({ appPage: page }) => {
  await pageData(page);
  const platformCss = page.locator('link[rel="stylesheet"][href$="/dist/platform.css"]');
  const prefix = new URL("./", await platformCss.evaluate(link => link.href));
  const licenses = await page.evaluate(async url => {
    const platform = await import(url);
    return { latin: platform.fontLicenseUrl, cjk: platform.cjkFontLicenseUrl };
  }, new URL("platform.js", prefix).href);
  const provenance = require("../../node_modules/@sarmg/web-fonts/dist/provenance.json");
  const fontCss = readFileSync(require("node:path").join(__dirname, "../../node_modules/@sarmg/web-fonts/dist/fonts.css"), "utf8");
  expect(fontCss).toContain('font-family:"Sarmg Maple Bootstrap"');
  for (const source of Object.keys(provenance.assets).filter(name => (name.endsWith(".woff2") && fontCss.includes(`./${name}`)) || name.endsWith(".txt"))) {
    const name = source.split("/").at(-1);
    // Vite may coalesce identical license bytes into a single emitted asset.
    const url = source === "CJK-LICENSE.txt" ? licenses.cjk
      : ["OFL.txt", "NORMAL-LICENSE.txt"].includes(source) ? licenses.latin
        : new URL(name, prefix).href;
    const response = await page.context().request.get(url);
    expect(response.status()).toBe(200);
    expect(response.headers()["cache-control"]).toContain("immutable");
    expect(response.headers()["x-content-type-options"]).toBe("nosniff");
    expect(response.headers()["content-type"]).toContain(name.endsWith("woff2") ? "font/woff2" : "text/plain");
    expect(await response.body()).toEqual(readFileSync(require("node:path").join(__dirname, "../../node_modules/@sarmg/web-fonts/dist", source)));
  }
  await page.evaluate(() => document.fonts.ready);
  expect(await page.evaluate(() => document.fonts.check('16px "Sarmg Maple"'))).toBe(true);
  expect(await page.locator("body").evaluate(element => getComputedStyle(element).fontFamily)).toContain("Sarmg Maple");
  expect(await page.locator("#dufs-root").evaluate(root => root.classList.contains("sarmg-font-bootstrap"))).toBe(false);
  const css = await page.context().request.get(new URL("platform.css", prefix).href);
  const cssText = await css.text();
  expect(cssText).not.toContain("data:image");
  const icons = [...new Set([...cssText.matchAll(/foundation-icon-[a-f0-9]{64}\.svg/gu)].map(match => match[0]))];
  expect(icons).toHaveLength(3);
  for (const icon of icons) {
    const response = await page.context().request.get(new URL(icon, prefix).href);
    expect(response.status()).toBe(200);
    expect(response.headers()["content-type"]).toContain("image/svg+xml");
    expect(response.headers()["cache-control"]).toContain("immutable");
  }
  const script = await page.context().request.get(new URL("platform.js", prefix).href);
  expect((await script.body()).byteLength).toBeLessThanOrEqual(512 * 1024);
  expect(await script.text()).not.toMatch(/sourceMappingURL/u);
  await expect(page.locator('#dufs-root[data-dufs-renderer="react"] > header')).toHaveCount(1);
  expect(await page.locator("#dufs-root").evaluate(root => Object.keys(root).some(key => key.startsWith("__reactContainer$")))).toBe(true);
});

test("React Profile 的移动明暗主题通过 WCAG AA", async ({ axePage: page }) => {
  await login(page);
  await page.setViewportSize({ width: 390, height: 844 });
  for (const colorScheme of ["light", "dark"]) {
    await page.emulateMedia({ colorScheme, reducedMotion: "reduce" });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const results = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"]).analyze();
    expect(results.violations).toEqual([]);
  }
});

test("Foundation 外观统一顶部项目名、等高图标和全宽文件内容", async ({ appPage: page }, testInfo) => {
  await expect(page.locator(".sarmg-page-header .sarmg-product-identity")).toHaveText("Dufs");
  await expect(page.locator(".sarmg-header-navigation a[aria-current=page]")).toHaveText("Files");
  await expect(page.locator(".sarmg-instance-sidebar")).toHaveCount(0);
  for (const width of [1280, 768, 320]) {
    await page.setViewportSize({ width, height: 900 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const main = await page.locator(".main").boundingBox();
    expect(main.x).toBeLessThanOrEqual(32);
    expect(main.width).toBeGreaterThan(width - 65);
    const responsiveHeader = await page.locator(".sarmg-page-header").boundingBox();
    const responsiveFirstContent = await page.locator(".file-toolbar").boundingBox();
    expect(Math.abs(responsiveFirstContent.y - (responsiveHeader.y + responsiveHeader.height) - 16)).toBeLessThan(2);
  }
  await page.setViewportSize({ width: 1280, height: 900 });
  const menuItems = page.locator(".dufs-file-actions > :is(.dufs-root-navigation, .upload-file, .upload-folder, .new-folder, .new-file, .searchbar)");
  await expect(menuItems).toHaveCount(6);
  const menuBar = page.locator(".sarmg-page-header .sarmg-header-actions");
  expect(await menuItems.evaluateAll(nodes => nodes.every(node => {
    return node.closest(".dufs-file-actions")?.parentElement?.matches(".sarmg-header-actions");
  }))).toBe(true);
  expect(await page.locator(".file-toolbar .dufs-file-actions")).toHaveCount(0);
  const menuBarBox = await menuBar.boundingBox();
  const headerBox = await page.locator(".sarmg-page-header").boundingBox();
  expect(menuBarBox).not.toBeNull();
  expect(headerBox).not.toBeNull();
  expect(menuBarBox.y).toBeGreaterThanOrEqual(headerBox.y);
  expect(menuBarBox.y + menuBarBox.height).toBeLessThanOrEqual(headerBox.y + headerBox.height);
  const button = page.getByRole("button", { name: "Switch to dark mode", exact: true });
  await button.click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByRole("button", { name: "Switch to light mode", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  for (const icon of await page.locator(".sarmg-header-actions button > svg").all()) {
    expect(await icon.evaluate(svg => Math.abs(svg.getBoundingClientRect().height - Number.parseFloat(getComputedStyle(svg.parentElement).fontSize)))).toBeLessThan(1);
  }
  expect(await page.locator(".paths-table").evaluate(table => getComputedStyle(table).borderCollapse)).toBe("collapse");
  await page.screenshot({ path: testInfo.outputPath("react-foundation-desktop.png") });
});

test("React 主题更新不重建上传队列或重复发送上传", async ({ appPage: page }) => {
  let releaseUpload;
  let reachedUpload;
  const gate = new Promise(resolve => { releaseUpload = resolve; });
  const reached = new Promise(resolve => { reachedUpload = resolve; });
  let requests = 0;
  await page.route("**/react-theme-upload.txt", async route => {
    if (route.request().method() === "PUT") {
      requests++;
      reachedUpload();
      await gate;
    }
    await route.continue();
  });
  try {
    await selectFiles(page, "#file", [{ name: "react-theme-upload.txt", mimeType: "text/plain", buffer: Buffer.from("React keeps the upload alive") }]);
    await reached;
    const row = await page.locator(".upload-status").elementHandle();
    expect(row).not.toBeNull();
    await page.getByRole("button", { name: "Switch to dark mode", exact: true }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await page.getByRole("button", { name: "Switch to light mode", exact: true }).click();
    expect(await row.evaluate(element => element.isConnected && element === document.querySelector(".upload-status"))).toBe(true);
    expect(requests).toBe(1);
  } finally { releaseUpload(); }
  await expect(page.locator(".upload-status")).toHaveAttribute("aria-label", "react-theme-upload.txt: upload complete");
  expect(requests).toBe(1);
});
