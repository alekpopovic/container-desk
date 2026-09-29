import { expect, test } from "@playwright/test";
import fixtures from "../fixtures/ipc.json" with { type: "json" };
for (const invalid of [false, true])
  test(`startup resource limits are bounded and visible (invalid=${invalid})`, async ({
    page,
  }) => {
    await page.addInitScript(
      ({ fixtures, invalid }) => {
        Reflect.set(window, "isTauri", true);
        Reflect.set(window, "__TAURI_INTERNALS__", {
          async invoke(command: string) {
            if (command === "get_resource_limits")
              return {
                limits: {
                  logLines: invalid ? 90000 : 1000,
                  logBytes: 262144,
                  statsHistory: 60,
                  activeHosts: 0,
                  concurrentJobs: 2,
                },
                configurationIgnored: false,
              };
            if (command === "get_preferences") return fixtures.preferences;
            if (command === "get_workspace_mode") return fixtures.liveWorkspace;
            if (command === "app_version") return { version: "fixture" };
            if (command === "get_ssh_config_path")
              return { path: "/fixture/config" };
            throw { code: "feature_unavailable" };
          },
        });
      },
      { fixtures, invalid },
    );
    await page.goto("/#/settings");
    const panel = page.getByRole("region", { name: "Resource limits" });
    if (invalid) {
      await expect(panel).toContainText("20,000 lines");
      await expect(panel.getByRole("alert")).toContainText(
        "Safe default limits",
      );
    } else {
      await expect(panel).toContainText("1,000 lines / 0.25 MiB");
      await expect(panel).toContainText("Active live hosts: 0");
      await expect(panel).toContainText("Concurrent SSH jobs per transport: 2");
      await expect(panel.getByRole("alert")).toHaveCount(0);
    }
  });
