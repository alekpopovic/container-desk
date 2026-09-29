import { expect, test } from "@playwright/test";
test.beforeEach(async ({ page }) => {
  page.on("pageerror", (error) =>
    console.error("Image fixture page error:", error.message),
  );
  await page.goto("/tests/ui/fixture.html?state=images");
  await expect(
    page.getByText("Read-only images for the selected daemon.", {
      exact: false,
    }),
  ).toBeVisible({ timeout: 15000 });
});
test("deduplicated tags, bounded image pages, masked metadata and exact reference navigation", async ({
  page,
}) => {
  await expect(page.locator(".image-choice")).toHaveCount(50);
  await expect(page.locator(".image-detail")).toContainText(
    "sha256:" + "c".repeat(64),
  );
  await expect(page.locator(".image-detail")).toContainText(
    "same:latest, same:stable",
  );
  await expect(page.locator(".image-detail img")).toHaveCount(0);
  await expect(page.locator(".image-detail")).toContainText("Masked");
  await page.getByRole("button", { name: "Reference host 1" }).click();
  await expect(page.getByLabel("Opened image reference")).toHaveText(
    "image-host-1:" + "a".repeat(64),
  );
  await page.getByRole("button", { name: "Next images" }).click();
  await expect(page.locator(".image-choice")).toHaveCount(3);
  await page
    .getByRole("searchbox", { name: "Search images" })
    .fill("same:stable");
  await expect(page.locator(".image-choice")).toHaveCount(1);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
test("real filter request displays no tags without offering deletion", async ({
  page,
}) => {
  await page.getByRole("checkbox", { name: "Dangling images only" }).check();
  await expect(page.locator(".image-choice")).toHaveCount(1);
  await expect(page.locator(".image-detail")).toContainText("No tags");
  await expect(page.locator(".image-detail")).toContainText(
    "No container references",
  );
  await expect(
    page.getByRole("button", { name: /delete|remove|pull/i }),
  ).toHaveCount(0);
});
test("host change rejects late same-tag response and refresh failure marks stale data", async ({
  page,
}) => {
  await expect(page.locator(".image-detail")).toContainText(
    "sha256:" + "c".repeat(64),
  );
  await page.getByRole("button", { name: "Fixture hold images" }).click();
  await page.getByRole("button", { name: "Refresh images" }).click();
  await page.getByRole("button", { name: "Fixture switch image host" }).click();
  await expect(page.locator(".image-detail")).toContainText(
    "sha256:" + "f".repeat(64),
  );
  await page.getByRole("button", { name: "Fixture release images" }).click();
  await expect(page.locator(".image-detail")).not.toContainText(
    "sha256:" + "c".repeat(64),
  );
  await page.getByRole("button", { name: "Fixture image failure" }).click();
  await page.getByRole("button", { name: "Refresh images" }).click();
  await expect(page.getByRole("alert")).toContainText("Stale image snapshot");
  await expect(
    page.getByRole("button", { name: "Reference host 2" }),
  ).toBeDisabled();
});
