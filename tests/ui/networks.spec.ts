import { expect, test } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  await page.goto("/tests/ui/fixture.html?state=networks");
  await expect(
    page.getByText("Read-only network metadata.", { exact: false }),
  ).toBeVisible({ timeout: 15000 });
});
test("dual-stack network, unknown endpoints, masking and bounded pages remain usable", async ({
  page,
}) => {
  await expect(page.locator(".network-choice")).toHaveCount(50);
  await expect(page.locator(".network-detail")).toContainText("fd36::/64");
  await expect(page.locator(".network-detail")).toContainText("10.36.0.2/24");
  await expect(
    page.getByRole("button", { name: "Unnamed endpoint" }),
  ).toBeDisabled();
  await expect(page.locator(".network-detail")).toContainText(
    "deleted, stale or non-container endpoint",
  );
  await expect(page.locator(".network-detail img")).toHaveCount(0);
  await expect(page.locator(".network-detail")).toContainText(
    "password: Masked",
  );
  await page.getByRole("button", { name: "Reference host 1" }).click();
  await expect(page.getByLabel("Opened network reference")).toHaveText(
    "network-host-1:" + "a".repeat(64),
  );
  await page.getByRole("button", { name: "Next networks" }).click();
  await expect(page.locator(".network-choice")).toHaveCount(3);
  await page.getByRole("searchbox", { name: "Search networks" }).fill("host");
  await expect(page.locator(".network-choice")).toHaveCount(1);
  await expect(page.locator(".network-detail")).toContainText(
    "No IPAM configuration reported.",
  );
  await expect(page.locator(".network-detail")).toContainText(
    "0 endpoint(s) reported",
  );
  await page.getByRole("searchbox", { name: "Search networks" }).fill("none");
  await expect(page.locator(".network-detail")).toContainText(
    "Attachments were not reported by this driver.",
  );
  await expect(
    page.getByRole("button", {
      name: /create network|remove|delete|prune|connect endpoint/i,
    }),
  ).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
test("missing container does not erase its reported endpoint or enable a wrong link", async ({
  page,
}) => {
  await expect(
    page.getByRole("button", { name: "Reference host 1" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Fixture stale endpoints" }).click();
  await page.getByRole("button", { name: "Refresh networks" }).click();
  await expect(
    page.getByRole("button", { name: "Reference host 1" }),
  ).toBeDisabled();
  await expect(page.locator(".network-detail")).toContainText(
    "2 endpoint(s) reported",
  );
  await expect(page.locator(".network-detail")).toContainText(
    "optional metadata was malformed",
  );
});
test("late previous-host result is discarded and failed refresh disables attachment navigation", async ({
  page,
}) => {
  await expect(
    page.getByRole("button", { name: "Reference host 1" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Fixture hold networks" }).click();
  await page.getByRole("button", { name: "Refresh networks" }).click();
  await page
    .getByRole("button", { name: "Fixture switch network host" })
    .click();
  await expect(
    page.getByRole("button", { name: "Reference host 2" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Fixture release networks" }).click();
  await expect(
    page.getByRole("button", { name: "Reference host 1" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Fixture network failure" }).click();
  await page.getByRole("button", { name: "Refresh networks" }).click();
  await expect(page.getByRole("alert")).toContainText("Stale network snapshot");
  await expect(
    page.getByRole("button", { name: "Reference host 2" }),
  ).toBeDisabled();
});
