import { expect, test } from "@playwright/test";
test("inspect masks values, reveals explicitly, clears on reconnect and handles deletion", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=inspect");
  const details = page.getByRole("region", {
    name: "Inspected container details",
  });
  await expect(details).toContainText("Sensitive values masked");
  await expect(details).toContainText("137");
  await expect(details).toContainText("fd00::2");
  await expect(details).toContainText("<img src=x onerror=alert(1)>");
  await expect(details.locator("img")).toHaveCount(0);
  await expect(details).not.toContainText("synthetic-ui-secret");
  await details
    .getByRole("button", { name: "Reveal sensitive values" })
    .click();
  await expect(
    page.getByText("Fixture reveal pending", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Finish fixture reveal" }).click();
  await expect(details).toContainText("synthetic-ui-secret");
  await expect(details).toContainText("synthetic-ui-label");
  await details.getByRole("button", { name: "Hide sensitive values" }).click();
  await expect(details).not.toContainText("synthetic-ui-secret");
  await details
    .getByRole("button", { name: "Reveal sensitive values" })
    .click();
  await expect(details).toContainText("synthetic-ui-secret");
  await page.getByRole("button", { name: "Reconnect fixture session" }).click();
  await expect(details).toContainText("Sensitive values masked");
  await expect(details).not.toContainText("synthetic-ui-secret");
  await page.getByRole("button", { name: "Delete fixture container" }).click();
  await details.getByRole("button", { name: "Refresh details" }).click();
  await expect(details.getByRole("alert")).toContainText("no longer exists");
  await expect(details).not.toContainText("SYNTHETIC_TOKEN");
});
test("late explicit reveal cannot populate a different selected container", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=inspect");
  await page.getByRole("button", { name: "Reveal sensitive values" }).click();
  await expect(
    page.getByText("Fixture reveal pending", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Switch fixture container" }).click();
  const details = page.getByRole("region", {
    name: "Inspected container details",
  });
  await expect(details).toContainText("Sensitive values masked");
  await page.getByRole("button", { name: "Finish fixture reveal" }).click();
  await expect(page.getByText("Fixture ready", { exact: true })).toBeVisible();
  await expect(details).toContainText("Sensitive values masked");
  await expect(details).not.toContainText("synthetic-ui-secret");
});
