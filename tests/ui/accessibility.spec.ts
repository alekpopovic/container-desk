import { expect, test } from "@playwright/test";

test("modal confirmation traps keys, starts on cancel and restores the trigger without dispatch", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=management");
  await page
    .getByRole("button", { name: "Enable management", exact: true })
    .click();
  const trigger = page.getByRole("button", {
    name: "Restart container",
    exact: true,
  });
  await trigger.focus();
  await page.keyboard.press("Enter");
  const modal = page.getByRole("dialog", { name: "Confirm container action" });
  await expect(
    modal.getByRole("button", { name: "Cancel action" }),
  ).toBeFocused();
  for (let n = 0; n < 5; n++) {
    await page.keyboard.press(n % 2 ? "Shift+Tab" : "Tab");
    expect(
      await modal.evaluate((element) =>
        element.contains(document.activeElement),
      ),
    ).toBe(true);
  }
  await page.keyboard.press("Escape");
  await expect(modal).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await expect(page.getByLabel("Mutation count")).toHaveText("0");
});

test("shortcuts and accessible paging survive 200 percent text and reduced motion", async ({
  page,
}, info) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/tests/ui/fixture.html?state=containers");
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "175%";
  });
  await page.keyboard.press("Control+f");
  await expect(
    page.getByRole("searchbox", { name: "Search containers" }),
  ).toBeFocused();
  await page.keyboard.press("Tab"); // State filter
  await page.keyboard.press("Tab"); // Refresh
  await page.keyboard.press("Tab"); // Paged table
  await expect(
    page.getByRole("checkbox", { name: "Use paged table" }),
  ).toBeFocused();
  await page.keyboard.press("Space");
  await page.keyboard.press("Tab"); // First-page Previous is disabled
  await expect(
    page.getByRole("button", { name: "Next containers" }),
  ).toBeFocused();
  for (let n = 0; n < 41; n++) await page.keyboard.press("Enter");
  await expect(page.getByText("Rows 985–1000 of 1000")).toBeVisible();
  expect(await page.locator("[data-container-id]").count()).toBe(16);
  const row = page.getByRole("button", { name: "workload-0999", exact: true });
  await row.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("complementary", { name: "Resource details" }),
  ).toContainText("workload-0999");
  const heights = await page
    .locator("[data-container-id]")
    .first()
    .evaluate((element) => ({
      height: element.getBoundingClientRect().height,
      font: parseFloat(getComputedStyle(element).fontSize),
    }));
  expect(heights.height).toBeGreaterThanOrEqual(112);
  expect(heights.font).toBeGreaterThanOrEqual(22);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.keyboard.press("Control+r");
  await expect(
    page.getByRole("button", { name: "Refresh containers" }),
  ).toBeEnabled();
  if (["wide-light", "minimum-dark"].includes(info.project.name))
    await page.screenshot({
      path: `docs/verification/044-browser/${info.project.name}-200-text.png`,
      fullPage: true,
    });
});
