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

test("pause and text selection hold rendering while bounded stream continues; clear and search stay usable", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=logs");
  await page.getByRole("button", { name: "Start logs", exact: true }).click();
  await page
    .getByRole("button", { name: "Deliver 1024 fixture lines" })
    .click();
  await expect(page.getByText(/1024 fixture lines sent/)).toBeVisible();
  await expect(page.locator(".log-preview")).toContainText("fixture-line-1023");
  expect(await page.locator(".log-preview pre").count()).toBeLessThanOrEqual(
    32,
  );
  await page
    .getByRole("button", { name: "Pause display", exact: true })
    .click();
  const before = await page.locator(".log-preview pre").allTextContents();
  await page.getByRole("button", { name: "Deliver export fixture" }).click();
  await expect(page.getByText(/Retained: 1027 lines/)).toBeVisible();
  await expect(page.locator(".log-preview pre")).toHaveText(before);
  await expect(
    page.getByRole("button", { name: "Stop logs", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "Resume display", exact: true })
    .click();
  await expect(page.locator(".log-preview")).toContainText(
    "third chosen čćž link [controls removed]",
  );
  await page
    .locator(".log-preview pre")
    .last()
    .evaluate((node) => {
      const range = document.createRange();
      range.selectNodeContents(node);
      document.getSelection()?.removeAllRanges();
      document.getSelection()?.addRange(range);
    });
  await expect(page.getByText(/Text selection holds/)).toBeVisible();
  const held = await page
    .locator(".log-preview")
    .evaluate((node) => node.scrollTop);
  // Keyboard/programmatic fixture delivery does not click away the selected text.
  await page
    .getByRole("button", { name: "Deliver 1024 fixture lines" })
    .evaluate((node: HTMLButtonElement) => node.click());
  await expect(page.getByText(/2048 fixture lines sent/)).toBeVisible();
  expect(
    await page.locator(".log-preview").evaluate((node) => node.scrollTop),
  ).toBe(held);
  await page.evaluate(() => document.getSelection()?.removeAllRanges());
  await expect(page.getByText(/Text selection holds/)).toHaveCount(0);
  await page
    .getByRole("searchbox", { name: "Search logs", exact: true })
    .fill("x".repeat(1000));
  await expect(
    page.getByRole("searchbox", { name: "Search logs", exact: true }),
  ).toHaveValue("x".repeat(256));
  await page.getByRole("button", { name: "Clear view", exact: true }).click();
  await expect(page.getByText(/Retained: 0 lines/)).toBeVisible();
  await page
    .getByRole("searchbox", { name: "Search logs", exact: true })
    .fill("");
  await page.getByRole("button", { name: "Deliver export fixture" }).click();
  await expect(page.getByText(/Retained: 3 lines/)).toBeVisible();
  await expect(page.locator(".log-preview")).not.toContainText("fixture-line-");
});

test("selected export and copy use exactly the sanitized visible order", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/tests/ui/fixture.html?state=logs");
  await page.getByRole("button", { name: "Start logs", exact: true }).click();
  await page.getByRole("button", { name: "Deliver export fixture" }).click();
  await page
    .getByRole("checkbox", { name: "Show timestamps", exact: true })
    .uncheck();
  await page
    .getByRole("searchbox", { name: "Search logs", exact: true })
    .fill("chosen");
  await page
    .getByRole("button", { name: "Select matching lines", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Copy selected lines", exact: true })
    .click();
  const expected = "first chosen\nthird chosen čćž link [controls removed]";
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    expected,
  );
  await page
    .getByRole("button", { name: "Export selected lines", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toContainText(
    "Logs may contain application secrets",
  );
  await page
    .getByRole("button", { name: "Save selected logs…", exact: true })
    .click();
  await expect(page.getByText("Saved 2 selected lines.")).toBeVisible();
  await page
    .getByRole("button", { name: "Read fixture export", exact: true })
    .click();
  const exported = JSON.parse(
    await page.getByLabel("Fixture export").innerText(),
  );
  expect(exported.lines.join("\n")).toBe(expected);
  expect(exported.containerId).toBe("a".repeat(64));
  expect(exported.secretsAcknowledged).toBe(true);
  expect(exported.lines.join("\n")).not.toContain("unrelated");
  expect(exported.lines.join("\n")).not.toContain("https://secret");
});
