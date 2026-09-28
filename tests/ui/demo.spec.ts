import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
const fixtures = JSON.parse(
  readFileSync(new URL("../fixtures/ipc.json", import.meta.url), "utf8"),
);

test("explicit demo displays states, IPv6 and Compose with a persistent label", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("status", { name: "Demo mode" })).toHaveCount(0);
  await page.getByRole("button", { name: "Open demo" }).click();
  await expect(page.getByRole("status", { name: "Demo mode" })).toBeVisible();
  await expect(page.getByText("demo-web", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("cell", { name: "running · healthy", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("cell", { name: "exited", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("cell", { name: "restarting", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("cell", { name: "running · unhealthy", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("[::1]:8080 → 80/tcp", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("demo-stack / web", { exact: true }),
  ).toBeVisible();
  await expect(page.locator("img")).toHaveCount(0);
  for (const route of [
    "Compose",
    "Images",
    "Volumes",
    "Networks",
    "Settings",
    "Containers",
  ]) {
    await page
      .getByRole("navigation", { name: "Resources" })
      .getByRole("link", { name: route, exact: true })
      .click();
    await expect(page.getByRole("status", { name: "Demo mode" })).toBeVisible();
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    const badge = await page
      .getByRole("status", { name: "Demo mode" })
      .boundingBox();
    expect(badge).not.toBeNull();
    expect((badge?.y ?? 0) + (badge?.height ?? 0)).toBeLessThanOrEqual(
      page.viewportSize()?.height ?? 0,
    );
  }
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "Exit demo" }).click();
  await expect(page.getByRole("status", { name: "Demo mode" })).toHaveCount(0);
  await expect(page.getByText("demo-web", { exact: true })).toHaveCount(0);
  await expect(
    page.getByText("Select a host to get started", { exact: true }),
  ).toBeVisible();
});

test("fixture errors clear rows and remain labeled DEMO until explicit exit", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open demo" }).click();
  const select = page.getByLabel("Demo scenario");
  for (const [scenario, text] of [
    ["empty", "No containers in this snapshot"],
    ["permission_failure", "This operation is not permitted."],
    ["invalid_json", "The desktop returned an invalid response."],
    ["huge_record", "The operation exceeded an application limit."],
    ["disconnect", "Host is offline"],
    ["timeout", "The command exceeded its deadline."],
  ]) {
    await select.selectOption(scenario ?? "");
    await expect(page.getByText(text ?? "", { exact: true })).toBeVisible();
    await expect(page.getByText("demo-web", { exact: true })).toHaveCount(0);
    await expect(page.getByRole("status", { name: "Demo mode" })).toBeVisible();
  }
  await select.selectOption("standard");
  await expect(page.getByText("demo-web", { exact: true })).toBeVisible();
});

test("native bridge failure never activates browser demo as fallback", async ({
  page,
}) => {
  await page.addInitScript((fixtures) => {
    Reflect.set(window, "isTauri", true);
    Reflect.set(window, "__TAURI_INTERNALS__", {
      async invoke(command: string) {
        if (command === "app_version") return { version: "fixture-008" };
        if (command === "get_preferences") return fixtures.preferences;
        throw { code: "permission_denied" };
      },
    });
  }, fixtures);
  await page.goto("/");
  await expect(
    page.getByText("Workspace mode could not be loaded.", { exact: false }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Open demo" }).click();
  await expect(page.getByRole("alert")).toContainText("not permitted");
  await expect(page.getByRole("status", { name: "Demo mode" })).toHaveCount(0);
  await expect(page.getByText("demo-web", { exact: true })).toHaveCount(0);
});
