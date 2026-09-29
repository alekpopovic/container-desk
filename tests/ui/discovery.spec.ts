import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
const fixtures = JSON.parse(
  readFileSync(new URL("../fixtures/ipc.json", import.meta.url), "utf8"),
);

test("discovery waits for explicit browse and accepts a selected or manual alias without connection", async ({
  page,
}) => {
  await page.addInitScript((fixtures) => {
    Reflect.set(window, "isTauri", true);
    const calls: string[] = [];
    Reflect.set(window, "discoveryCalls", calls);
    Reflect.set(window, "__TAURI_INTERNALS__", {
      async invoke(
        command: string,
        args?: {
          request?: {
            configPath?: string;
            alias?: string;
            selection?: {
              alias: string;
              configPath: string;
              useDefaultConfig: boolean;
            };
          };
        },
      ) {
        calls.push(command);
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
        if (command === "app_version") return { version: "fixture-009" };
        if (command === "get_preferences") return fixtures.preferences;
        if (command === "get_workspace_mode") return fixtures.liveWorkspace;
        if (command === "get_ssh_config_path")
          return { path: fixtures.discovery.configPath };
        if (command === "discover_ssh_hosts")
          return args?.request?.configPath === "/missing/config"
            ? {
                configPath: "/missing/config",
                candidates: [],
                warnings: [
                  {
                    code: "missing_file",
                    source: "/missing/config",
                    line: null,
                  },
                ],
              }
            : fixtures.discovery;
        if (command === "check_ssh_access")
          return {
            selection: args?.request?.selection,
            status: "unknown_host_key",
            sshError: "Host key verification failed.",
          };
        if (command === "resolve_ssh_config")
          return {
            ...fixtures.effectiveSsh,
            selection: args?.request?.selection,
          };
        if (command === "select_ssh_alias")
          return {
            configPath:
              args?.request?.configPath || fixtures.discovery.configPath,
            alias: args?.request?.alias,
            useDefaultConfig: !args?.request?.configPath,
          };
        throw new Error("Unexpected fixture command");
      },
    });
  }, fixtures);
  await page.goto("/#/settings");
  await expect(
    page.getByText("Default config:", { exact: false }),
  ).toContainText("/fixture/.ssh/config");
  expect(
    await page.evaluate(() => Reflect.get(window, "discoveryCalls")),
  ).not.toContain("discover_ssh_hosts");
  await page.getByRole("button", { name: "Browse host candidates" }).click();
  await expect(page.getByLabel("Discovery notes")).toContainText("Wildcard");
  await page
    .getByRole("button", { name: "Select fixture-host", exact: true })
    .click();
  await expect(
    page.getByRole("status").filter({ hasText: "Selected fixture-host" }),
  ).toContainText("No connection was made");
  expect(
    await page.evaluate(() => Reflect.get(window, "discoveryCalls")),
  ).not.toContain("resolve_ssh_config");
  await expect(
    page.getByText("OpenSSH -G evaluates Match", { exact: false }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Resolve selected alias" }).click();
  await expect(page.getByLabel("Effective SSH configuration")).toContainText(
    "192.0.2.10",
  );
  await expect(page.getByLabel("Effective SSH configuration")).toContainText(
    "fixture-host",
  );
  await expect(page.getByLabel("Effective SSH configuration")).toContainText(
    "fixture-jump",
  );
  expect(
    await page.evaluate(() => Reflect.get(window, "discoveryCalls")),
  ).not.toContain("check_ssh_access");
  await page.getByRole("button", { name: "Check SSH access" }).click();
  await expect(page.getByLabel("SSH access result")).toContainText(
    "Verify its fingerprint independently",
  );
  await expect(page.getByLabel("SSH access result")).toContainText(
    "Host key verification failed.",
  );
  await page
    .getByText("Terminal setup and host trust", { exact: true })
    .click();
  await expect(
    page.getByText("Load encrypted keys using ssh-add", { exact: false }),
  ).toBeVisible();
  expect(
    (
      await page.evaluate(
        () => Reflect.get(window, "discoveryCalls") as string[],
      )
    ).filter((c) => c === "check_ssh_access"),
  ).toHaveLength(1);
  await page
    .getByLabel("SSH config path", { exact: true })
    .fill("/missing/config");
  await expect(page.getByLabel("Effective SSH configuration")).toHaveCount(0);
  await expect(page.getByLabel("SSH access result")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Select fixture-host", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Browse host candidates" }).click();
  await expect(
    page.getByText("No literal host candidates found.", { exact: false }),
  ).toBeVisible();
  await page
    .getByLabel("Manual SSH alias", { exact: true })
    .fill("-oProxyCommand=x");
  await expect(
    page.getByRole("button", { name: "Select manual alias" }),
  ).toBeDisabled();
  await page
    .getByLabel("Manual SSH alias", { exact: true })
    .fill("manual-lab-1");
  await page.getByRole("button", { name: "Select manual alias" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Selected manual-lab-1" }),
  ).toContainText("/missing/config");
  const calls = await page.evaluate(
    () => Reflect.get(window, "discoveryCalls") as string[],
  );
  expect(calls).not.toContain("connect_host");
  expect(calls).not.toContain("dependency_diagnostics");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});
