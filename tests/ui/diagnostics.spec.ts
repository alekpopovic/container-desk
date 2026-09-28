import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
const fixtures = JSON.parse(
  readFileSync(new URL("../fixtures/ipc.json", import.meta.url), "utf8"),
);

test("native diagnostics view explains runtime requirements and invalid override", async ({
  page,
}) => {
  await page.addInitScript((fixtures) => {
    Reflect.set(window, "isTauri", true);
    Reflect.set(window, "__TAURI_INTERNALS__", {
      async invoke(command: string) {
        if (command === "get_ssh_config_path")
          return { path: "/fixture/.ssh/config" };
        if (command === "app_version") return { version: "fixture-006" };
        if (command === "get_preferences") return fixtures.preferences;
        if (command === "get_workspace_mode") return fixtures.liveWorkspace;
        if (command === "dependency_diagnostics") return fixtures.diagnostics;
        if (command === "set_ssh_executable")
          return {
            preferences: null,
            ssh: {
              path: "/missing/ssh",
              status: "missing",
              version: null,
              message:
                "SSH was not found. Install OpenSSH or choose its absolute executable path.",
            },
          };
        throw new Error("Unexpected fixture command");
      },
    });
  }, fixtures);
  await page.goto("/#/settings");
  await expect(
    page.getByText(
      "Local Docker, jq, Python, Rust and Node are not required.",
      { exact: false },
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Run diagnostics" }).click();
  await expect(
    page.getByText("OpenSSH_fixture", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText("linux / x86_64", { exact: true })).toBeVisible();
  await expect(
    page.getByText("SSH_AUTH_SOCK is not set.", { exact: false }),
  ).toBeVisible();
  await page.getByLabel("OpenSSH executable override").fill("/missing/ssh");
  await page
    .getByRole("button", { name: "Validate and save SSH path" })
    .click();
  await expect(
    page.getByRole("status").filter({ hasText: "SSH was not found" }),
  ).toBeVisible();
  await expect(
    page.getByText("SSH executable preference saved.", { exact: true }),
  ).toHaveCount(0);
});
