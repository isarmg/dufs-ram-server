const { test, expect } = require('./fixtures');

test('人物图标修改单一管理员并保留文件访问', async ({ appPage: page }) => {
  test.slow();
  const originalUrl = page.url();
  const entry = page.getByRole('button', { name: 'Account settings', exact: true });
  await expect(entry).toBeVisible();
  await expect(page.getByRole('button', { name: /Create administrator|创建管理员/ })).toHaveCount(0);
  await entry.click();
  const dialog = page.getByRole('dialog', { name: 'Account settings', exact: true });
  await expect(dialog.getByLabel('Account name', { exact: true })).toHaveValue('frontend-test-0');
  await dialog.getByLabel('Account name', { exact: true }).fill('renamed-admin');
  await dialog.getByLabel('Current password', { exact: true }).fill('test-password');
  await dialog.getByLabel('New password', { exact: true }).fill('updated test password');
  await dialog.getByLabel('Confirm new password', { exact: true }).fill('updated test password');
  await dialog.getByRole('button', { name: 'Save account', exact: true }).click();
  await expect(page).toHaveURL(/\/__dufs__\/login$/);
  await page.getByLabel('Username', { exact: true }).fill('renamed-admin');
  await page.getByLabel('Password', { exact: true }).fill('updated test password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await expect(page.locator('.index-page')).toBeVisible();
  await page.goto(originalUrl);
  await expect(page.getByRole('link', { name: 'existing-folder', exact: true })).toBeVisible();
  // Return the isolated server's fixture account for the remaining cases.
  await page.getByRole('button', { name: 'Account settings', exact: true }).click();
  await dialog.getByLabel('Account name', { exact: true }).fill('frontend-test-0');
  await dialog.getByLabel('Current password', { exact: true }).fill('updated test password');
  await dialog.getByLabel('New password', { exact: true }).fill('test-password');
  await dialog.getByLabel('Confirm new password', { exact: true }).fill('test-password');
  await dialog.getByRole('button', { name: 'Save account', exact: true }).click();
  await expect(page).toHaveURL(/\/__dufs__\/login$/);
});
