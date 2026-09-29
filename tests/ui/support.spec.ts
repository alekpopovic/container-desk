import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
const fixtures = JSON.parse(
  readFileSync(new URL("../fixtures/ipc.json", import.meta.url), "utf8"),
);
test("support export previews inert text and clearing requires an explicit confirmation", async ({
  page,
}) => {
  await page.addInitScript((fixtures) => {
    Reflect.set(window, "isTauri", true);
    Reflect.set(window, "supportCalls", []);
    Reflect.set(window, "__TAURI_INTERNALS__", {
      async invoke(command: string, args: unknown) {
        (Reflect.get(window, "supportCalls") as unknown[]).push([
          command,
          args,
        ]);
        if (command === "get_resource_limits")
          return {
            limits: {
              logLines: 20000,
              logBytes: 8388608,
              statsHistory: 360,
              activeHosts: 1,
              concurrentJobs: 4,
            },
            configurationIgnored: false,
          };
        if (command === "app_version") return { version: "fixture" };
        if (command === "get_preferences") return fixtures.preferences;
        if (command === "get_workspace_mode") return fixtures.liveWorkspace;
        if (command === "get_ssh_config_path")
          return { path: "/fixture/config" };
        if (command === "prepare_support_report")
          return {
            id: "a".repeat(32),
            expiresInSeconds: 300,
            report: JSON.stringify({
              schemaVersion: 1,
              privacy: '<img src=x onerror="window.reportExecuted=true">',
            }),
          };
        if (command === "save_support_report") return false;
        if (command === "clear_support_data") return null;
        throw { code: "permission_denied", message: "fixture" };
      },
    });
  }, fixtures);
  await page.goto("/#/settings");
  await expect(
    page.getByRole("button", { name: "Save reviewed report…" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Prepare support preview" }).click();
  await expect(page.getByLabel("Support report preview")).toHaveValue(/<img/);
  expect(
    await page.evaluate(() => Reflect.get(window, "reportExecuted")),
  ).toBeUndefined();
  await page.getByRole("button", { name: "Save reviewed report…" }).click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Save cancelled; no report written." }),
  ).toBeVisible();
  await expect(page.getByLabel("Support report preview")).toBeVisible();
  await page
    .getByRole("button", { name: "Clear local troubleshooting data…" })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  expect(
    await page.evaluate(
      () =>
        (Reflect.get(window, "supportCalls") as [string, unknown][]).filter(
          (c) => c[0] === "clear_support_data",
        ).length,
    ),
  ).toBe(0);
  await page
    .getByRole("button", { name: "Clear local troubleshooting data…" })
    .click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Clear local history and preview" })
    .click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Local activity and prepared report cleared." }),
  ).toBeVisible();
  await expect(page.getByLabel("Support report preview")).toHaveCount(0);
  const calls = await page.evaluate(() =>
    (Reflect.get(window, "supportCalls") as [string, unknown][]).filter(
      (c) => c[0] === "clear_support_data" || c[0] === "save_support_report",
    ),
  );
  expect(calls).toEqual([
    ["save_support_report", { request: { previewId: "a".repeat(32) } }],
    ["clear_support_data", { request: { confirmed: true } }],
  ]);
});
