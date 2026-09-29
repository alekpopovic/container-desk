import { expect, test } from "@playwright/test";
test("Compose projects isolate same service names, show unverified remote metadata and share container navigation", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=compose");
  await expect(
    page.getByText("Remote Compose plugin available."),
  ).toBeVisible();
  const first = page.getByRole("article", { name: "Project alpha" });
  await expect(first).toContainText("web");
  await expect(first).toContainText("unverified");
  await expect(first).toContainText("/remote/deleted/compose.yml");
  await expect(first.locator("img,a")).toHaveCount(0);
  await page
    .getByRole("button", { name: "beta 1 container instances" })
    .click();
  const second = page.getByRole("article", { name: "Project beta" });
  await expect(second).toContainText("web");
  await expect(second).toContainText("No configuration files reported.");
  await expect(second).not.toContainText("alpha-web");
  await second.getByRole("button", { name: "beta-web-1" }).click();
  await expect(page.getByLabel("Opened container")).toHaveText(
    "2".padStart(64, "0"),
  );
  await page.getByRole("button", { name: "Fixture plugin absent" }).click();
  await expect(
    page.getByText(
      "Remote Compose plugin absent; grouping uses container labels.",
    ),
  ).toBeVisible();
  await expect(
    second.getByRole("button", { name: "beta-web-1" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Fixture disconnect" }).click();
  await expect(
    page.getByText("No Compose data", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("article")).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});
