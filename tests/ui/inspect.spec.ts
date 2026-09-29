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
  await details.getByRole("tab", { name: "Networks", exact: true }).click();
  await expect(details).toContainText("fd00::2");
  await expect(details).toContainText("<img src=x onerror=alert(1)>");
  await expect(details.locator("img")).toHaveCount(0);
  await details.getByRole("tab", { name: "Environment", exact: true }).click();
  await expect(details).not.toContainText("synthetic-ui-secret");
  await details
    .getByRole("button", { name: "Reveal sensitive values" })
    .click();
  await expect(
    page.getByText("Fixture reveal pending", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Finish fixture reveal" }).click();
  await expect(details).toContainText("synthetic-ui-secret");
  await details.getByRole("tab", { name: "Labels", exact: true }).click();
  await expect(details).toContainText("synthetic-ui-label");
  await expect(details.locator("a,img")).toHaveCount(0);
  await details.getByRole("tab", { name: "Environment", exact: true }).click();
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
  await page.getByRole("tab", { name: "Environment", exact: true }).click();
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

test("detail tabs separate health and port exposure, wrap long paths and copy only chosen metadata", async ({
  page,
  context,
}, info) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/tests/ui/fixture.html?state=inspect");
  const panel = page.getByRole("tabpanel");
  await expect(panel).toContainText("running");
  await expect(panel).toContainText("unhealthy");
  await expect(panel).toContainText("out-of-memory kill");
  await panel
    .getByRole("button", { name: "Copy container id", exact: true })
    .click();
  await expect(
    panel.getByText("Copied container id.", { exact: true }),
  ).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "a".repeat(64),
  );
  await page.getByRole("tab", { name: "Overview", exact: true }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("tab", { name: "Ports", exact: true }),
  ).toBeFocused();
  await expect(panel).toHaveAccessibleName("Ports");
  await expect(panel).toContainText("Exposed container ports");
  await expect(panel).toContainText("53/udp");
  await expect(panel).toContainText("[::]:8080 → 80/tcp");
  await expect(panel).toContainText("127.0.0.1:18080 → 80/tcp");
  await page.getByRole("tab", { name: "Mounts", exact: true }).click();
  await expect(panel).toContainText("long-directory-".repeat(30));
  await expect(panel).toContainText("Read-only");
  await panel
    .getByRole("button", { name: "Copy mount source", exact: true })
    .click();
  await expect(
    panel.getByText("Copied mount source.", { exact: true }),
  ).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "/fixture/" + "long-directory-".repeat(30),
  );
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("mounts.png"),
    fullPage: true,
  });
  await page.getByRole("tab", { name: "Mounts", exact: true }).focus();
  await page.keyboard.press("End");
  await expect(
    page.getByRole("tab", { name: "Environment", exact: true }),
  ).toBeFocused();
  await expect(panel.getByRole("button", { name: /^Copy / })).toHaveCount(0);
  await panel.getByRole("button", { name: "Reveal sensitive values" }).click();
  await page.getByRole("button", { name: "Finish fixture reveal" }).click();
  await expect(panel).toContainText("synthetic-ui-secret");
  await expect(panel.locator("img,a")).toHaveCount(0);
  await page.goto("/tests/ui/fixture.html?state=inspect&emptyLabels=1");
  await page.getByRole("tab", { name: "Labels", exact: true }).click();
  await expect(panel).toContainText("No labels reported.");
});

test("inventory bursts preserve the inspect tab, mark old details and fence a pending reveal", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=inspect");
  const details = page.getByRole("region", {
    name: "Inspected container details",
  });
  await expect(details).toContainText("Sensitive values masked");
  await details.getByRole("tab", { name: "Environment", exact: true }).click();
  for (let n = 0; n < 5; n++)
    await page
      .getByRole("button", { name: "Refresh fixture inventory" })
      .click();
  await expect(
    details.getByRole("tab", { name: "Environment", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(details).toContainText("earlier snapshot");
  await expect(details).toContainText("SYNTHETIC_TOKEN");
  await details
    .getByRole("button", { name: "Reveal sensitive values" })
    .click();
  await expect(
    page.getByText("Fixture reveal pending", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Refresh fixture inventory" }).click();
  await page.getByRole("button", { name: "Finish fixture reveal" }).click();
  await expect(details).not.toContainText("synthetic-ui-secret");
  await details.getByRole("button", { name: "Refresh details" }).click();
  await expect(details).toContainText("Sensitive values masked");
  await expect(details).not.toContainText("earlier snapshot");
  await details
    .getByRole("button", { name: "Reveal sensitive values" })
    .click();
  await expect(details).toContainText("synthetic-ui-secret");
  await page.getByRole("button", { name: "Refresh fixture inventory" }).click();
  await expect(details).not.toContainText("synthetic-ui-secret");
});
