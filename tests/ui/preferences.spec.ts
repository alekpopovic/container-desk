import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import type { PreferencesSnapshot } from "../../src/lib/ipc/generated";
const fixture = JSON.parse(
  readFileSync(new URL("../fixtures/ipc.json", import.meta.url), "utf8"),
);

for (const failSave of [false, true]) {
  test(`saved theme and recovery feedback, save fails = ${failSave}`, async ({
    page,
  }) => {
    const initial: PreferencesSnapshot = {
      ...fixture.preferences,
      preferences: { ...fixture.preferences.preferences, theme: "dark" },
      notice: "recovered_previous",
    };
    await page.addInitScript(
      ({ initial, failSave }) => {
        let snapshot = initial;
        Reflect.set(window, "isTauri", true);
        Reflect.set(window, "__TAURI_INTERNALS__", {
          async invoke(
            command: string,
            args: {
              request?: {
                theme: "system" | "light" | "dark";
                expectedRevision: number;
              };
            },
          ) {
            if (command === "app_version") return { version: "fixture-005" };
            if (command === "get_preferences") return snapshot;
            if (command === "get_workspace_mode")
              return { mode: "live", scenario: null, scope: null, host: null };
            if (command !== "set_theme" || !args.request)
              throw new Error("Unexpected fixture command");
            if (failSave) throw { code: "storage_unavailable" };
            snapshot = {
              ...snapshot,
              preferences: {
                ...snapshot.preferences,
                theme: args.request.theme,
                revision: args.request.expectedRevision + 1,
              },
            };
            return snapshot;
          },
        });
      },
      { initial, failSave },
    );
    await page.goto("/#/settings");
    await expect(
      page.getByRole("status").filter({ hasText: "Damaged settings" }),
    ).toBeVisible();
    await expect(
      page.getByRole("radio", { name: "Dark", exact: true }),
    ).toBeChecked();
    await page.getByRole("radio", { name: "Light", exact: true }).click();
    if (failSave) {
      await expect(page.getByRole("alert")).toContainText(
        "original files were retained",
      );
      await expect(
        page.getByRole("radio", { name: "Dark", exact: true }),
      ).toBeChecked();
      await expect(
        page.getByRole("radio", { name: "Light", exact: true }),
      ).toBeDisabled();
    } else {
      await expect(
        page.getByRole("radio", { name: "Light", exact: true }),
      ).toBeChecked();
      await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    }
  });
}
