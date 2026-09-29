import { expect, test } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  await page.goto("/tests/ui/fixture.html?state=volumes");
  await expect(
    page.getByText("Read-only volume metadata.", { exact: false }),
  ).toBeVisible({ timeout: 15000 });
});
test("volume metadata and mounts use bounded pages, mask values and share container navigation", async ({
  page,
}) => {
  await expect(page.locator(".volume-choice")).toHaveCount(50);
  await expect(page.locator(".volume-detail")).toContainText("Read-only mount");
  await expect(page.locator(".volume-detail img")).toHaveCount(0);
  await expect(page.locator(".volume-detail")).toContainText(
    "password: Masked",
  );
  await page.getByRole("button", { name: "Reference host 1" }).click();
  await expect(page.getByLabel("Opened volume reference")).toHaveText(
    "volume-host-1:" + "a".repeat(64),
  );
  await page.getByRole("button", { name: "Next volumes" }).click();
  await expect(page.locator(".volume-choice")).toHaveCount(3);
  await page
    .getByRole("searchbox", { name: "Search volumes" })
    .fill("plugin-data");
  await expect(page.locator(".volume-choice")).toHaveCount(1);
  await expect(page.locator(".volume-detail")).toContainText(
    "vendor/plugin:latest",
  );
  await expect(page.locator(".volume-detail")).toContainText("Not reported");
  await expect(page.locator(".volume-detail")).toContainText(
    "No container references were observed in this snapshot.",
  );
  await expect(page.locator(".volume-detail")).toContainText(
    "not proof that deletion is safe",
  );
  await expect(
    page.getByRole("button", { name: /browse|delete|remove|prune/i }),
  ).toHaveCount(0);
  await expect(page.locator(".volume-detail a")).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
test("container disappearance stays incomplete and cannot be called unused", async ({
  page,
}) => {
  await expect(page.locator(".volume-detail")).toContainText(
    "mount reference(s) observed",
  );
  await page
    .getByRole("button", { name: "Fixture container disappeared" })
    .click();
  await page.getByRole("button", { name: "Refresh volumes" }).click();
  await expect(page.locator(".volume-detail")).toContainText(
    "Incomplete reference snapshot: 1",
  );
  await expect(page.locator(".volume-detail")).not.toContainText(
    "No container references were observed",
  );
});
test("late old-host volume response is discarded and failed refresh disables references", async ({
  page,
}) => {
  await expect(
    page.getByRole("button", { name: "Reference host 1" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Fixture hold volumes" }).click();
  await page.getByRole("button", { name: "Refresh volumes" }).click();
  await page
    .getByRole("button", { name: "Fixture switch volume host" })
    .click();
  await expect(
    page.getByRole("button", { name: "Reference host 2" }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Fixture release volumes" }).click();
  await expect(
    page.getByRole("button", { name: "Reference host 1" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Fixture volume failure" }).click();
  await page.getByRole("button", { name: "Refresh volumes" }).click();
  await expect(page.getByRole("alert")).toContainText("Stale volume snapshot");
  await expect(
    page.getByRole("button", { name: "Reference host 2" }),
  ).toBeDisabled();
});
