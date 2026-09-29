import { expect, test, type Page } from "@playwright/test";
async function select(page: Page, n: number) {
  await page.getByRole("searchbox").fill(`batch-${String(n).padStart(2, "0")}`);
  await page.getByRole("checkbox", { name: /^Select batch-/ }).check();
}
test.beforeEach(async ({ page }) => {
  await page.goto("/tests/ui/fixture.html?state=batch");
});
test("exact batch confirmation keeps success, disappearance and permission results visible", async ({
  page,
}) => {
  for (const n of [1, 2, 3]) await select(page, n);
  await page.getByRole("button", { name: "Enable batch management" }).click();
  await page
    .getByRole("button", { name: "Start selected", exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: "Confirm container batch" });
  await expect(dialog).toContainText("Batch host 1");
  for (const n of [1, 2, 3])
    await expect(dialog).toContainText(n.toString(16).padStart(64, "0"));
  await page.getByRole("button", { name: "Confirm selected action" }).click();
  const results = page.getByLabel("Individual action results");
  await expect(results.locator("li")).toHaveCount(3);
  await expect(results).toContainText("succeeded");
  await expect(results).toContainText("This operation is not permitted");
  await expect(results).toContainText("no longer exists");
  await expect(page.getByLabel("Batch calls")).toHaveText("1");
});
test("pending cancellation preserves the dispatched result", async ({
  page,
}) => {
  for (const n of [1, 2]) await select(page, n);
  await page.getByRole("button", { name: "Fixture hold batch" }).click();
  await page.getByRole("button", { name: "Enable batch management" }).click();
  await page
    .getByRole("button", { name: "Stop selected", exact: true })
    .click();
  await page.getByRole("button", { name: "Confirm selected action" }).click();
  await page.getByRole("button", { name: "Cancel pending actions" }).click();
  await expect(
    page.getByRole("button", { name: "Clear batch selection" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Fixture release batch" }).click();
  await expect(page.locator('[data-outcome="succeeded"]')).toContainText(
    "command dispatched",
  );
  await expect(page.locator('[data-outcome="cancelled"]')).toContainText(
    "command not dispatched",
  );
  await expect(page.getByLabel("Batch calls")).toHaveText("1");
});
test("selection is bounded and cleared on host change; running removal unavailable", async ({
  page,
}) => {
  for (let n = 1; n <= 20; n++) await select(page, n);
  await page.getByRole("searchbox").fill("batch-21");
  await expect(
    page.getByRole("checkbox", { name: /^Select batch-/ }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Fixture switch host" }).click();
  await expect(page.getByLabel("Selected container actions")).toHaveCount(0);
  await page.getByRole("checkbox", { name: /^Select batch-/ }).check();
  await page.getByRole("button", { name: "Enable batch management" }).click();
  await expect(
    page.getByRole("button", { name: "Remove selected stopped containers" }),
  ).toBeDisabled();
});
test("removal uses separate acknowledgement and retains removed target result", async ({
  page,
}) => {
  await select(page, 1);
  await page.getByRole("button", { name: "Enable batch management" }).click();
  await page
    .getByRole("button", { name: "Remove selected stopped containers" })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Confirm stopped-container removal" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Confirm selected action" }),
  ).toBeDisabled();
  await page
    .getByRole("checkbox", {
      name: "I confirm removal of the listed stopped containers.",
    })
    .check();
  await page.getByRole("button", { name: "Confirm selected action" }).click();
  await expect(page.locator('[data-outcome="succeeded"]')).toContainText(
    "batch-01",
  );
  await expect(
    page.getByRole("checkbox", { name: /^Select batch-01/ }),
  ).toHaveCount(0);
});
