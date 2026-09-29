import { test, expect } from "@playwright/test";
test("live log gap marker is visible, hostile text stays plain and unmount cancels", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=logs");
  await page.getByRole("button", { name: "Start logs", exact: true }).click();
  await page.getByRole("button", { name: "Deliver fixture overflow" }).click();
  await expect(page.getByText(/43210 records dropped/)).toBeVisible();
  await expect(page.locator(".log-preview")).toContainText(
    "<img src=x onerror=alert(1)> synthetic Unicode čćž [truncated]",
  );
  await expect(page.locator(".log-preview img, .log-preview a")).toHaveCount(0);
  await page.getByRole("button", { name: "Stop logs", exact: true }).click();
  await page
    .getByRole("button", { name: "Read fixture cancellations" })
    .click();
  await expect(page.getByText("1 fixture cancellations")).toBeVisible();
  await page.getByRole("button", { name: "Start logs", exact: true }).click();
  await page.getByRole("button", { name: "Unmount fixture logs" }).click();
  await page
    .getByRole("button", { name: "Read fixture cancellations" })
    .click();
  await expect(page.getByText("2 fixture cancellations")).toBeVisible();
});
