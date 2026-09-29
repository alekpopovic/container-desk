import { expect, test } from "@playwright/test";
test("demo inventory separates duplicate names, filters favorites and retains identity through explicit actions", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open demo" }).click();
  await page.getByRole("button", { name: "Add host" }).click();
  const inventory = page.getByRole("region", {
    name: "Saved host inventory",
    exact: true,
  });
  await inventory.getByRole("button", { name: "Browse aliases" }).click();
  await inventory
    .getByRole("button", { name: "Use demo-direct", exact: true })
    .click();
  await inventory
    .getByLabel("Display name", { exact: true })
    .fill("Duplicate name");
  const labels = inventory.getByLabel("Labels (comma separated)");
  await labels.pressSequentially("one,two");
  await inventory
    .getByRole("button", { name: "Save host", exact: true })
    .click();
  const details = inventory.getByRole("region", {
    name: "Selected saved host",
  });
  await expect(details).toContainText("Disconnected");
  const firstId = await details.locator("code").textContent();
  await inventory
    .getByRole("button", { name: "Connect saved host", exact: true })
    .click();
  await expect(details).toContainText("Ready");
  await expect(details).toContainText("Direct");
  await expect(details).toContainText("unix:///demo/docker.sock");
  await inventory
    .getByRole("button", { name: "New host", exact: true })
    .click();
  await inventory.getByRole("button", { name: "Browse aliases" }).click();
  await inventory
    .getByRole("button", { name: "Use demo-jump", exact: true })
    .click();
  await inventory
    .getByLabel("Display name", { exact: true })
    .fill("Duplicate name");
  await inventory.getByLabel("Host group", { exact: true }).fill("prod");
  await inventory.getByLabel("Favorite host", { exact: true }).check();
  await inventory
    .getByRole("button", { name: "Save host", exact: true })
    .click();
  const secondId = await details.locator("code").textContent();
  expect(firstId).not.toEqual(secondId);
  await expect(details).toContainText("Disconnected");
  await inventory
    .getByRole("button", { name: "Connect saved host", exact: true })
    .click();
  await expect(details).toContainText("demo-bastion");
  await expect(details).toContainText("Ready");
  await inventory.getByLabel("Filter hosts").selectOption("Favorites");
  const cards = inventory.getByRole("list", {
    name: "Saved hosts",
    exact: true,
  });
  await expect(cards.getByRole("listitem")).toHaveCount(1);
  await expect(cards).toContainText("demo-jump");
  await inventory
    .getByRole("button", { name: "Disconnect saved host", exact: true })
    .click();
  await expect(details).toContainText("Disconnected");
  await expect(details).not.toContainText("unix:///demo/docker.sock");
  await inventory
    .getByRole("button", { name: "Remove from app", exact: true })
    .click();
  await expect(details).toHaveCount(0);
  await inventory.getByLabel("Filter hosts").selectOption("All");
  await expect(cards.getByRole("listitem")).toHaveCount(1);
  await expect(cards).toContainText("one, two");
  await cards
    .getByRole("button", { name: "Duplicate name · demo-direct", exact: true })
    .click();
  await expect(details.locator("code")).toHaveText(firstId ?? "");
  await inventory
    .getByRole("button", { name: "Remove from app", exact: true })
    .click();
  await expect(cards.getByRole("listitem")).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

test("live inventory preserves saved config policy and keeps the host selected after a connection error", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const host = {
      id: `h_${"1".repeat(32)}`,
      alias: "fixture-direct",
      displayName: "Fixture host",
      group: "staging",
      labels: [],
      favorite: false,
      readOnly: true,
      ssh: {
        alias: "fixture-direct",
        configPath: "/fixture/default-config",
        useDefaultConfig: true,
      },
      docker: { executable: null, context: null, sudo: false },
    };
    const saved = {
      writable: true,
      notice: null,
      preferences: {
        schemaVersion: 3,
        revision: 0,
        theme: "system",
        hosts: [host],
        selectedHostId: host.id,
        trustedConfigPath: null,
        sshExecutableOverride: null,
      },
    };
    const inventory: Record<string, unknown> = {
      mode: "live",
      saved,
      connection: null,
    };
    let connects = 0;
    Reflect.set(window, "isTauri", true);
    Reflect.set(window, "__TAURI_INTERNALS__", {
      async invoke(
        command: string,
        args?: { request?: { draft?: typeof host; expectedRevision?: number } },
      ) {
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
        if (command === "app_version") return { version: "fixture-017" };
        if (command === "get_workspace_mode")
          return { mode: "live", scenario: null, scope: null, host: null };
        if (command === "get_preferences") return structuredClone(saved);
        if (command === "get_host_inventory") return structuredClone(inventory);
        if (command === "save_host") {
          const draft = args?.request?.draft;
          if (
            !draft ||
            draft.ssh.configPath !== "/fixture/default-config" ||
            draft.ssh.useDefaultConfig !== true
          )
            throw new Error("config policy changed");
          saved.preferences.hosts[0] = { ...host, ...draft };
          saved.preferences.revision += 1;
          return structuredClone(inventory);
        }
        if (command === "connect_inventory_host") {
          connects += 1;
          Reflect.set(window, "inventoryConnectCount", connects);
          inventory.connection = {
            hostId: host.id,
            selection: host.ssh,
            effective: null,
            dockerOptions: host.docker,
            token: {
              sessionId: `s_${"2".repeat(32)}`,
              sessionGeneration: connects,
            },
            state: "error",
            durations: [{ stage: "resolve", durationMs: 1 }],
            diagnostic: { stage: "resolve", code: "resolution_failed" },
            hasJump: false,
            transportMode: "unconnected",
            docker: null,
          };
          return structuredClone(inventory);
        }
        throw new Error(`Unexpected command ${command}`);
      },
    });
  });
  await page.goto("/#/hosts");
  const inventory = page.getByRole("region", {
    name: "Saved host inventory",
    exact: true,
  });
  await inventory
    .getByRole("button", { name: "Fixture host · fixture-direct", exact: true })
    .click();
  await inventory.getByLabel("Favorite host", { exact: true }).check();
  await inventory
    .getByRole("button", { name: "Save host changes", exact: true })
    .click();
  await expect(
    inventory.getByRole("button", {
      name: "★ Fixture host · fixture-direct",
      exact: true,
    }),
  ).toBeVisible();
  expect(
    await page.evaluate(() => Reflect.get(window, "inventoryConnectCount")),
  ).toBeUndefined();
  await inventory
    .getByRole("button", { name: "Connect saved host", exact: true })
    .click();
  const details = inventory.getByRole("region", {
    name: "Selected saved host",
  });
  await expect(details).toContainText("fixture-direct");
  await expect(details).toContainText("OpenSSH could not resolve");
  await expect(
    details.getByRole("button", { name: "Retry connection", exact: true }),
  ).toBeEnabled();
  expect(
    await page.evaluate(() => Reflect.get(window, "inventoryConnectCount")),
  ).toBe(1);
  await details
    .getByRole("button", { name: "Retry connection", exact: true })
    .click();
  await expect
    .poll(() =>
      page.evaluate(() => Reflect.get(window, "inventoryConnectCount")),
    )
    .toBe(2);
});
