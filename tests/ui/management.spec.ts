import { expect, test } from "@playwright/test";
test("explicit management, exact confirmation, one dispatch and observed health", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=management");
  await expect(
    page.getByRole("button", { name: "Restart container", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Fixture hold mutation" }).click();
  await page.getByRole("button", { name: "Enable management" }).click();
  await page
    .getByRole("button", { name: "Restart container", exact: true })
    .click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("Disposable fixture host");
  await expect(dialog).toContainText("a".repeat(64));
  await expect(dialog.locator("img")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Confirm action", exact: true })
    .dblclick();
  await expect(page.getByLabel("Mutation count")).toHaveText("1");
  await expect(
    page.getByRole("button", { name: "Restart container", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Fixture release mutation" }).click();
  await expect(page.getByRole("status")).toContainText(
    "Observed state: running; health: starting",
  );
  await expect(page.getByLabel("Refresh count")).toHaveText("1");
});
test("unknown outcome requires read refresh and never replays after reconnect", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=management");
  await page.getByRole("button", { name: "Fixture unknown outcome" }).click();
  await page.getByRole("button", { name: "Enable management" }).click();
  await page
    .getByRole("button", { name: "Stop container", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Confirm action", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText("Unknown outcome");
  await expect(
    page.getByRole("button", { name: "Start container", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Refresh action state" }).click();
  await expect(page.getByRole("status")).toContainText(
    "Observed state: exited",
  );
  await page.getByRole("button", { name: "Fixture reconnect" }).click();
  await expect(page.getByRole("status")).toContainText(
    "Previous action has an unknown outcome",
  );
  await page.getByRole("button", { name: "Enable management" }).click();

  await expect(
    page.getByRole("button", { name: "Start container", exact: true }),
  ).toBeDisabled();
  await expect(page.getByLabel("Mutation count")).toHaveText("1");
});
test("expired confirmation cannot submit", async ({ page }) => {
  await page.goto("/tests/ui/fixture.html?state=management");
  await page.getByRole("button", { name: "Fixture short expiry" }).click();
  await page.getByRole("button", { name: "Enable management" }).click();
  await page
    .getByRole("button", { name: "Start container", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Confirm action", exact: true }),
  ).toBeDisabled();
  await expect(page.getByRole("dialog")).toContainText("Confirmation expired");
  await expect(page.getByLabel("Mutation count")).toHaveText("0");
});
