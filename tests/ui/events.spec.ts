import { expect, test } from "@playwright/test";
test("event bursts stay bounded; snapshots determine deletion, gap reconnect reads again and scope disposal stops", async ({
  page,
}) => {
  await page.clock.install();
  await page.goto("/tests/ui/fixture.html?state=events");
  await expect(page.getByLabel("Starts", { exact: true })).toHaveText("1");
  await page.clock.runFor(600);
  await expect(page.getByLabel("Reads", { exact: true })).toHaveText("2");
  for (let i = 0; i < 20; i++)
    await page.getByRole("button", { name: "Fixture event burst" }).click();
  await expect(page.getByLabel("Rows", { exact: true })).toHaveText("1");
  await page.clock.runFor(2100);
  await expect(page.getByLabel("Reads", { exact: true })).toHaveText("3");
  await expect(page.getByLabel("Rows", { exact: true })).toHaveText("1");
  await page.getByRole("button", { name: "Fixture delete silently" }).click();
  await page.getByRole("button", { name: "Fixture event gap" }).click();
  await expect(page.locator("p[role=status]")).toContainText("Event gap");
  await page.clock.runFor(2100);
  await expect(page.getByLabel("Starts", { exact: true })).toHaveText("2");
  await expect(page.getByLabel("Rows", { exact: true })).toHaveText("0");
  await expect(page.getByLabel("Selected", { exact: true })).toHaveText("none");
  await expect(
    page.getByLabel("Recovery since", { exact: true }),
  ).toContainText("1760000000.");
  await page.getByRole("button", { name: "Fixture disconnect" }).click();
  const reads = await page.getByLabel("Reads", { exact: true }).textContent();
  await page.getByRole("button", { name: "Fixture event burst" }).click();
  await page.clock.runFor(10000);
  await expect(page.getByLabel("Reads", { exact: true })).toHaveText(
    reads ?? "",
  );
});
