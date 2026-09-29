import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
const fixtures = JSON.parse(
  readFileSync(new URL("../fixtures/ipc.json", import.meta.url), "utf8"),
);
test("switching during probe and cancellation reject old completions", async ({
  page,
}) => {
  await page.addInitScript((fixtures) => {
    Reflect.set(window, "isTauri", true);
    let generation = 0;
    let polls = 0;
    let current: Record<string, unknown> | null = null;
    const calls: string[] = [];
    Reflect.set(window, "sessionCalls", calls);
    Reflect.set(window, "__TAURI_INTERNALS__", {
      async invoke(
        command: string,
        args?: {
          request?: {
            alias?: string;
            selection?: Record<string, unknown>;
            docker?: Record<string, unknown>;
            token?: { sessionId: string; sessionGeneration: number };
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
        if (command === "app_version") return { version: "fixture-014" };
        if (command === "get_preferences") return fixtures.preferences;
        if (command === "get_workspace_mode") return fixtures.liveWorkspace;
        if (command === "get_ssh_config_path")
          return { path: "/fixture/config" };
        if (command === "select_ssh_alias")
          return {
            alias: args?.request?.alias,
            configPath: "/fixture/config",
            useDefaultConfig: true,
          };
        if (command === "begin_ssh_session") {
          generation += 1;
          polls = 0;
          current = {
            token: {
              sessionId: `s_${generation.toString(16).padStart(32, "0")}`,
              sessionGeneration: generation,
            },
            selection: args?.request?.selection,
            state: "resolving",
            durations: [],
            diagnostic: null,
            hasJump: true,
            hostId: null,
            effective: null,
            transportMode: "direct_fallback",
            dockerOptions: args?.request?.docker,
            docker: null,
          };
          return { ...current };
        }
        if (command === "get_ssh_session") {
          polls += 1;
          const selected = current?.selection as { alias: string };
          if (selected.alias === "old-host") {
            if (polls === 1)
              return {
                ...current,
                state: "probing",
                durations: [
                  { stage: "resolve", durationMs: 5 },
                  { stage: "authenticate", durationMs: 10 },
                ],
              };
            const old = { ...current, state: "ready" };
            return new Promise((resolve) =>
              Reflect.set(window, "finishOldProbe", () => resolve(old)),
            );
          }
          return {
            ...current,
            state: generation <= 3 ? "ready" : "resolving",
            docker:
              generation <= 3
                ? {
                    status: "ready",
                    context: "rootless",
                    endpoint: "unix:///run/user/1000/docker.sock",
                    endpointKind: "unix",
                    clientVersion: "29.fixture",
                    serverVersion: "29.fixture",
                    daemonId: "fixture-daemon",
                    os: "linux",
                    rootless: true,
                    compose: "absent",
                    composeVersion: null,
                    sudo: true,
                  }
                : null,
          };
        }
        if (command === "disconnect_ssh_session") {
          generation += 1;
          return {
            ...current,
            token: { ...args?.request?.token, sessionGeneration: generation },
            state: "disconnected",
          };
        }
        throw new Error("Unexpected fixture command");
      },
    });
  }, fixtures);
  await page.goto("/#/settings");
  await page.getByLabel("Manual SSH alias", { exact: true }).fill("old-host");
  await page.getByRole("button", { name: "Select manual alias" }).click();
  const panel = page.getByRole("region", { name: "SSH connection session" });
  await expect(panel).toContainText("Disconnected");
  expect(
    await page.evaluate(() => Reflect.get(window, "sessionCalls")),
  ).not.toContain("begin_ssh_session");
  await page.getByRole("button", { name: "Connect selected host" }).click();
  await expect(panel).toContainText("Checking remote capabilities");
  await expect
    .poll(() =>
      page.evaluate(() => typeof Reflect.get(window, "finishOldProbe")),
    )
    .toBe("function");
  await page.getByLabel("Manual SSH alias", { exact: true }).fill("new-host");
  await page.getByRole("button", { name: "Select manual alias" }).click();
  await page.getByLabel("Remote Docker context (optional)").fill("rootless");
  await page
    .getByLabel("Use existing noninteractive sudo access (sudo -n)")
    .check();
  await page.getByRole("button", { name: "Connect selected host" }).click();
  await expect(panel.getByRole("status", { exact: true }).first()).toHaveText(
    "Ready",
  );
  const capabilities = panel.getByRole("region", {
    name: "Docker capabilities",
  });
  await expect(capabilities).toContainText("fixture-daemon");
  await expect(capabilities).toContainText("unix:///run/user/1000/docker.sock");
  await expect(capabilities).toContainText("absent");
  await expect(
    page.getByLabel("Remote Docker context (optional)"),
  ).toBeDisabled();
  await page.evaluate(() => Reflect.get(window, "finishOldProbe")());
  await expect(panel).toContainText("Session generation 3");
  await expect(panel).toContainText("SSH transport: direct fallback.");
  await expect(panel).toContainText(
    "each command opens its own strict SSH connection.",
  );
  await page.getByRole("button", { name: "Disconnect", exact: true }).click();
  await expect(panel.getByRole("status").first()).toHaveText("Disconnected");
  await page.getByRole("button", { name: "Connect selected host" }).click();
  await expect(panel).toContainText("Resolving configuration");
  await page.getByRole("button", { name: "Cancel connection" }).click();
  await expect(panel.getByRole("status").first()).toHaveText("Disconnected");
});
