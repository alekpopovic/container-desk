import { test, expect } from "@playwright/test";
test("statistics show multicore values, gaps, configured sampling and stop when inactive or disconnected", async ({
  page,
}) => {
  await page.clock.install({ time: new Date("2026-09-29T00:00:00Z") });
  await page.goto("/tests/ui/fixture.html?state=stats");
  await expect(page.locator(".stats-values")).toContainText("234.5%");
  await expect(page.locator(".stats-values")).toContainText("1.17 KiB");
  await expect(page.getByText(/CPU can exceed 100%/)).toBeVisible();
  await expect(page.locator(".stats-chart polyline")).toHaveCount(2);
  await page.clock.pauseAt(new Date("2026-09-29T00:00:02Z"));
  await page.getByRole("button", { name: "Fixture inactive window" }).click();
  await expect(
    page.locator(".container-stats").getByRole("status"),
  ).toContainText("Window inactive");
  const calls = await page.getByLabel("Stats fixture calls").innerText();
  await page.clock.runFor(60000);
  await expect(page.getByLabel("Stats fixture calls")).toHaveText(calls);
  await page.getByRole("button", { name: "Fixture active window" }).click();
  await expect(page.getByLabel("Stats fixture calls")).toHaveText(
    String(Number(calls) + 1),
  );
  await page.getByLabel("Stats interval").selectOption("30000");
  await page
    .getByRole("button", { name: "Fixture stopped", exact: true })
    .click();
  await page.clock.runFor(5100); // Previously scheduled interval may complete once.
  await expect(
    page.locator(".container-stats").getByRole("status"),
  ).toContainText("Container stopped · gap");
  await expect(page.locator(".stats-values")).not.toContainText("0%");
  await expect(page.locator(".stats-values")).toContainText("Unavailable");
  const after = await page.getByLabel("Stats fixture calls").innerText();
  await page.clock.runFor(10000);
  await expect(page.getByLabel("Stats fixture calls")).toHaveText(after);
  await page
    .getByRole("button", { name: "Fixture disappeared", exact: true })
    .click();
  await page.clock.runFor(21000);
  await expect(
    page.locator(".container-stats").getByRole("status"),
  ).toContainText("Container disappeared · gap");
  await page
    .getByRole("button", { name: "Fixture disconnect", exact: true })
    .click();
  const disconnected = await page.getByLabel("Stats fixture calls").innerText();
  await page.clock.runFor(120000);
  await expect(page.getByLabel("Stats fixture calls")).toHaveText(disconnected);
  await expect(
    page.getByRole("region", { name: "Container resource statistics" }),
  ).toHaveCount(0);
});
