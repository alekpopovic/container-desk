import { expect, test, type Page } from "@playwright/test";

async function assertLayout(page: Page) {
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  for (const name of [
    "Containers",
    "Compose",
    "Images",
    "Volumes",
    "Networks",
    "Settings",
  ]) {
    const link = page
      .getByRole("navigation", { name: "Resources" })
      .getByRole("link", { name, exact: true });
    await expect(link).toBeVisible();
    const box = await link.boundingBox();
    expect(box).not.toBeNull();
    if (box) {
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(
        page.viewportSize()?.width ?? 0,
      );
    }
  }
}

function contrast(a: string, b: string) {
  const luminance = (color: string) => {
    const parts = [1, 3, 5].map((offset) => {
      const n = Number.parseInt(color.slice(offset, offset + 2), 16) / 255;
      return n <= 0.04045 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4;
    });
    return (
      (parts[0] ?? 0) * 0.2126 +
      (parts[1] ?? 0) * 0.7152 +
      (parts[2] ?? 0) * 0.0722
    );
  };
  const x = luminance(a),
    y = luminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}

async function assertContrast(page: Page) {
  const colors = await page.evaluate(() => {
    const style = getComputedStyle(document.documentElement);
    return Object.fromEntries(
      [
        "canvas",
        "surface",
        "sidebar",
        "foreground",
        "muted",
        "accent",
        "accent-soft",
        "focus",
        "offline",
        "info",
        "danger",
        "danger-soft",
      ].map((name) => [name, style.getPropertyValue(`--${name}`).trim()]),
    );
  });
  for (const background of ["canvas", "surface", "sidebar"]) {
    for (const text of ["foreground", "muted"])
      expect(
        contrast(colors[text] ?? "", colors[background] ?? ""),
      ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrast(colors.focus ?? "", colors[background] ?? ""),
    ).toBeGreaterThanOrEqual(3);
  }
  for (const [foreground, background] of [
    ["accent", "accent-soft"],
    ["offline", "surface"],
    ["info", "surface"],
    ["info", "canvas"],
    ["danger", "danger-soft"],
  ]) {
    expect(
      contrast(colors[foreground ?? ""] ?? "", colors[background ?? ""] ?? ""),
    ).toBeGreaterThanOrEqual(4.5);
  }
}

test("all routes remain usable with honest empty state and host identity", async ({
  page,
}, info) => {
  await page.goto("/");
  for (const name of [
    "Containers",
    "Compose",
    "Images",
    "Volumes",
    "Networks",
    "Settings",
  ]) {
    await page
      .getByRole("navigation", { name: "Resources" })
      .getByRole("link", { name, exact: true })
      .click();
    await expect(
      page.getByRole("heading", { name, exact: true, level: 2 }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "No host selected", level: 1 }),
    ).toBeVisible();
    if (name !== "Settings") {
      await expect(
        page.getByRole("complementary", { name: "Resource details" }),
      ).toContainText("No host selected");
      await expect(
        page.getByText("Select a host to get started", { exact: true }),
      ).toBeVisible();
    }
    await assertLayout(page);
  }
  await page.goBack();
  await expect(
    page.getByRole("heading", { name: "Networks", level: 2 }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Ungrouped" }).click();
  await expect(page.getByRole("button", { name: "Ungrouped" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.getByRole("button", { name: "Add host" }).click();
  await expect(
    page.getByRole("main").getByRole("heading", { name: "Hosts", level: 2 }),
  ).toBeVisible();
  await page.goto("/#/containers");
  await assertContrast(page);
  await page.screenshot({ path: info.outputPath("empty.png"), fullPage: true });
});

test("keyboard skip, route navigation and theme radios retain visible focus", async ({
  page,
}) => {
  await page.goto("/");
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: "Skip to content" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("main")).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(
    page.getByRole("link", { name: "Settings", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "Settings", level: 2 }),
  ).toBeVisible();
  await page.getByRole("radio", { name: "Use system" }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("radio", { name: "Light", exact: true }),
  ).toBeChecked();
  expect(
    await page
      .getByRole("radio", { name: "Light", exact: true })
      .evaluate((element) => getComputedStyle(element).outlineWidth),
  ).toBe("3px");
  await assertContrast(page);
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("radio", { name: "Dark", exact: true }),
  ).toBeChecked();
  await assertContrast(page);
  await page.getByRole("link", { name: "Containers", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
});

test("loading offline and error fixtures keep host context and untrusted text readable", async ({
  page,
}, info) => {
  for (const state of ["loading", "offline", "error"]) {
    await page.goto(`/tests/ui/fixture.html?state=${state}#/containers`);
    await expect(page.getByRole("heading", { level: 1 })).toContainText(
      "Fixture host",
    );
    await expect(
      page.getByRole("complementary", { name: "Resource details" }),
    ).toContainText("fixture-only");
    await assertLayout(page);
    await assertContrast(page);
    if (state === "loading")
      await expect(
        page.getByRole("region", { name: "Containers inventory" }),
      ).toHaveAttribute("aria-busy", "true");
    if (state === "offline")
      await expect(
        page.getByText("Host is offline", { exact: true }),
      ).toBeVisible();
    if (state === "error") {
      await expect(page.getByRole("alert")).toContainText('<img src="x"');
      await expect(page.locator("img")).toHaveCount(0);
    }
    await page.screenshot({
      path: info.outputPath(`${state}.png`),
      fullPage: true,
    });
  }
});
