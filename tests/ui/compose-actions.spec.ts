import { expect, test, type Page } from "@playwright/test";
async function configure(page: Page) {
  await page
    .getByLabel("Remote project directory", { exact: true })
    .fill("/srv/project's directory");
  await page
    .getByLabel("Ordered remote config files", { exact: false })
    .fill("/srv/project's directory/base file.yml\n/srv/override's.yml");
  await expect(
    page.getByRole("button", { name: "Verify remote project" }),
  ).toBeDisabled();
  await page.getByRole("checkbox").check();
  await page.getByRole("button", { name: "Verify remote project" }).click();
  await expect(page.locator(".compose-management [role=status]")).toContainText(
    "Verified remote configuration",
  );
}
test.beforeEach(async ({ page }) => {
  await page.goto("/tests/ui/fixture.html?state=compose-actions");
  await expect(
    page.getByText("Configure remote project actions", { exact: true }),
  ).toBeVisible({ timeout: 15000 });
});
test("explicit configuration, opt-in and full confirmation precede one project command", async ({
  page,
}) => {
  await configure(page);
  await expect(
    page.getByRole("button", { name: "Restart verified services" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Enable Compose management" }).click();
  await page.getByRole("button", { name: "Restart verified services" }).click();
  const dialog = page.getByRole("dialog", { name: "Confirm Compose action" });
  await expect(dialog).toContainText("Fixture host 1");
  await expect(dialog).toContainText("compose-1");
  await expect(dialog).toContainText("/srv/project's directory/base file.yml");
  await expect(dialog).toContainText("a".repeat(64));
  await expect(dialog).toContainText("b".repeat(64));
  await page.getByRole("button", { name: "Cancel Compose action" }).click();
  await expect(page.getByLabel("Compose dispatch count")).toHaveText("0");
  await page.getByRole("button", { name: "Restart verified services" }).click();
  await page.getByRole("button", { name: "Confirm Compose action" }).click();
  await expect(page.getByLabel("Compose dispatch count")).toHaveText("1");
  await expect(page.getByLabel("Observed Compose service state")).toContainText(
    "running",
  );
  await expect(
    page.getByRole("button", { name: "Restart verified services" }),
  ).toBeDisabled();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});
test("edits and host changes discard verification and configuration is never taken from labels", async ({
  page,
}) => {
  await configure(page);
  await page.getByLabel("Explicit project name").fill("wrong");
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await page.getByRole("checkbox").check();
  await page.getByRole("button", { name: "Verify remote project" }).click();
  await expect(page.locator(".compose-management [role=status]")).toContainText(
    "does not match",
  );
  await expect(
    page.getByRole("button", { name: "Restart verified services" }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: "Fixture switch Compose host" })
    .click();
  await expect(
    page.getByLabel("Remote project directory", { exact: true }),
  ).toHaveValue("");
  await expect(page.getByRole("checkbox")).not.toBeChecked();
  await expect(page.getByLabel("Compose dispatch count")).toHaveText("0");
});
test("unknown outcome is retained without replay and requires another explicit verification", async ({
  page,
}) => {
  await configure(page);
  await page.getByRole("button", { name: "Enable Compose management" }).click();
  await page
    .getByRole("button", { name: "Fixture unknown Compose outcome" })
    .click();
  await page.getByRole("button", { name: "Restart verified services" }).click();
  await page.getByRole("button", { name: "Confirm Compose action" }).click();
  await expect(page.locator(".compose-management [role=status]")).toContainText(
    "Unknown outcome",
  );
  await expect(page.getByLabel("Compose dispatch count")).toHaveText("1");
  await expect(
    page.getByRole("button", { name: "Restart verified services" }),
  ).toBeDisabled();
  await expect(page.getByLabel("Observed Compose service state")).toContainText(
    "running",
  );
});
