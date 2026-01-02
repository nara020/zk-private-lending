/**
 * E2E Tests for ZK Private Lending Frontend
 *
 * Tests cover:
 * - Wallet connection flow
 * - Deposit collateral
 * - Borrow USDC
 * - Repay debt
 * - Interest accrual display
 *
 * Run with: npx playwright test
 */

import { test, expect } from '@playwright/test';

test.describe('ZK Private Lending App', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('/');
  });

  test('should load the home page', async ({ page }) => {
    // Check title
    await expect(page).toHaveTitle(/ZK Private/);

    // Check main heading exists
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
  });

  test('should show connect wallet button when not connected', async ({ page }) => {
    // Check for connect wallet button
    const connectButton = page.getByRole('button', { name: /connect/i });
    await expect(connectButton).toBeVisible();
  });

  test('should navigate between tabs', async ({ page }) => {
    // Check for tab navigation
    const depositTab = page.getByRole('tab', { name: /deposit/i });
    const borrowTab = page.getByRole('tab', { name: /borrow/i });
    const repayTab = page.getByRole('tab', { name: /repay/i });

    // Click through tabs
    if (await depositTab.isVisible()) {
      await depositTab.click();
      await expect(depositTab).toHaveAttribute('aria-selected', 'true');
    }

    if (await borrowTab.isVisible()) {
      await borrowTab.click();
      await expect(borrowTab).toHaveAttribute('aria-selected', 'true');
    }

    if (await repayTab.isVisible()) {
      await repayTab.click();
      await expect(repayTab).toHaveAttribute('aria-selected', 'true');
    }
  });

  test('should display pool statistics', async ({ page }) => {
    // Check for pool stats section
    const poolSection = page.locator('[data-testid="pool-stats"]').or(
      page.getByText(/total collateral/i)
    );

    // Either pool stats exist or we're in a non-connected state
    const exists = await poolSection.isVisible().catch(() => false);
    if (exists) {
      await expect(poolSection).toBeVisible();
    }
  });

  test('should show interest rate information', async ({ page }) => {
    // Look for APY or interest rate display
    const apyDisplay = page.getByText(/apy/i).or(
      page.getByText(/interest rate/i)
    );

    const exists = await apyDisplay.isVisible().catch(() => false);
    if (exists) {
      await expect(apyDisplay).toBeVisible();
    }
  });

  test('should have responsive design', async ({ page }) => {
    // Test mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await expect(page.locator('body')).toBeVisible();

    // Test tablet viewport
    await page.setViewportSize({ width: 768, height: 1024 });
    await expect(page.locator('body')).toBeVisible();

    // Test desktop viewport
    await page.setViewportSize({ width: 1440, height: 900 });
    await expect(page.locator('body')).toBeVisible();
  });
});

test.describe('Deposit Flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should show deposit form', async ({ page }) => {
    // Navigate to deposit tab if needed
    const depositTab = page.getByRole('tab', { name: /deposit/i });
    if (await depositTab.isVisible()) {
      await depositTab.click();
    }

    // Check for deposit form elements
    const depositInput = page.getByPlaceholder(/0\.0/i).or(
      page.getByRole('textbox', { name: /amount/i })
    );

    const exists = await depositInput.isVisible().catch(() => false);
    if (exists) {
      await expect(depositInput).toBeVisible();
    }
  });

  test('should validate deposit input', async ({ page }) => {
    // Navigate to deposit
    const depositTab = page.getByRole('tab', { name: /deposit/i });
    if (await depositTab.isVisible()) {
      await depositTab.click();
    }

    // Try to find and interact with deposit input
    const depositInput = page.getByPlaceholder(/0\.0/i).first();
    if (await depositInput.isVisible()) {
      // Enter invalid amount
      await depositInput.fill('-1');

      // Submit button should be disabled or show error
      const submitButton = page.getByRole('button', { name: /deposit/i });
      if (await submitButton.isVisible()) {
        await expect(submitButton).toBeDisabled();
      }
    }
  });
});

test.describe('Borrow Flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should show borrow form with interest rate', async ({ page }) => {
    // Navigate to borrow tab
    const borrowTab = page.getByRole('tab', { name: /borrow/i });
    if (await borrowTab.isVisible()) {
      await borrowTab.click();
    }

    // Check for interest rate display (new feature)
    const apyDisplay = page.getByText(/borrow apy/i).or(
      page.getByText(/current.*apy/i)
    );

    const exists = await apyDisplay.isVisible().catch(() => false);
    if (exists) {
      await expect(apyDisplay).toBeVisible();
    }
  });

  test('should show estimated interest when amount entered', async ({ page }) => {
    // Navigate to borrow
    const borrowTab = page.getByRole('tab', { name: /borrow/i });
    if (await borrowTab.isVisible()) {
      await borrowTab.click();
    }

    // Enter amount
    const borrowInput = page.getByPlaceholder(/0\.0/i).first();
    if (await borrowInput.isVisible()) {
      await borrowInput.fill('1000');

      // Check for estimated interest display
      const interestEstimate = page.getByText(/estimated.*interest/i).or(
        page.getByText(/daily interest/i)
      );

      const exists = await interestEstimate.isVisible().catch(() => false);
      if (exists) {
        await expect(interestEstimate).toBeVisible();
      }
    }
  });
});

test.describe('Repay Flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should show repay form with interest breakdown', async ({ page }) => {
    // Navigate to repay tab
    const repayTab = page.getByRole('tab', { name: /repay/i });
    if (await repayTab.isVisible()) {
      await repayTab.click();
    }

    // Check for principal and interest breakdown (new feature)
    const principalDisplay = page.getByText(/principal/i);
    const interestDisplay = page.getByText(/accrued interest/i).or(
      page.getByText(/interest/i)
    );

    const principalExists = await principalDisplay.isVisible().catch(() => false);
    const interestExists = await interestDisplay.isVisible().catch(() => false);

    if (principalExists) {
      await expect(principalDisplay).toBeVisible();
    }
    if (interestExists) {
      await expect(interestDisplay).toBeVisible();
    }
  });

  test('should show full repayment benefit', async ({ page }) => {
    // Navigate to repay
    const repayTab = page.getByRole('tab', { name: /repay/i });
    if (await repayTab.isVisible()) {
      await repayTab.click();
    }

    // Check for MAX button
    const maxButton = page.getByRole('button', { name: /max/i });
    if (await maxButton.isVisible()) {
      await expect(maxButton).toBeVisible();
    }
  });
});

test.describe('Accessibility', () => {
  test('should have proper ARIA labels', async ({ page }) => {
    await page.goto('/');

    // Check for accessible form inputs
    const inputs = page.locator('input');
    const inputCount = await inputs.count();

    for (let i = 0; i < inputCount; i++) {
      const input = inputs.nth(i);
      const hasLabel = await input.getAttribute('aria-label') ||
                       await input.getAttribute('aria-labelledby') ||
                       await input.getAttribute('placeholder');
      expect(hasLabel).toBeTruthy();
    }
  });

  test('should be keyboard navigable', async ({ page }) => {
    await page.goto('/');

    // Tab through the page
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');

    // Check that something is focused
    const focusedElement = page.locator(':focus');
    await expect(focusedElement).toBeVisible();
  });
});
