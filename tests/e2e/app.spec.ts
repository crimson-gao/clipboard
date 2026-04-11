import { expect, test } from '@playwright/test';

test('renders the clipboard shell and settings modal', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByText('剪切板')).toBeVisible();
  await expect(
    page.getByRole('navigation', { name: '内容分类' }),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: '设置' })).toBeVisible();

  await page.getByRole('button', { name: '设置' }).click();

  await expect(page.getByRole('region', { name: '设置' })).toBeVisible();
  await expect(page.getByText('修改后会自动保存')).toBeVisible();
});
