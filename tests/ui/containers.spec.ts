import { expect, test } from "@playwright/test";
test("1000-row inventory stays bounded, filters quickly and preserves then clears full-ID selection", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=containers");
  const table = page.locator(".container-table");
  await expect(table).toHaveAttribute("aria-rowcount", "1001");
  expect(
    await table.locator("[data-container-id]").count(),
  ).toBeLessThanOrEqual(24);
  const before = await page.evaluate(() => performance.now());
  await page
    .getByRole("searchbox", { name: "Search containers" })
    .fill("workload-0999");
  await expect(table.locator("[data-container-id]")).toHaveCount(1);
  const duration = (await page.evaluate(() => performance.now())) - before;
  expect(duration).toBeLessThan(1000);
  console.log(`1000-row filter ${duration.toFixed(1)} ms; <=24 rendered rows`);
  await page
    .getByRole("button", { name: "workload-0999", exact: true })
    .click();
  const details = page.getByRole("complementary", { name: "Resource details" });
  await expect(details.locator("code")).toHaveText(
    (1000).toString(16).padStart(64, "0"),
  );
  await page.getByRole("button", { name: "Refresh containers" }).click();
  await expect(
    page.getByRole("button", { name: "Refresh containers" }),
  ).toBeEnabled();
  await expect(details).toContainText("workload-0999");
  const successful = await page.locator("time").getAttribute("datetime");
  await page.getByRole("button", { name: "Fixture toggle failure" }).click();
  await page.getByRole("button", { name: "Refresh containers" }).click();
  await expect(page.getByRole("alert")).toContainText("Stale snapshot");
  await expect(page.locator("time")).toHaveAttribute(
    "datetime",
    successful ?? "",
  );
  await expect(details).toContainText("last successful snapshot");
  await page.getByRole("button", { name: "Fixture toggle failure" }).click();
  await page.getByRole("button", { name: "Fixture remove selected" }).click();
  await page.getByRole("button", { name: "Refresh containers" }).click();
  await expect(details).toContainText("No container selected");
  await expect(table.locator("[data-container-id]")).toHaveCount(0);
  await page.getByRole("searchbox", { name: "Search containers" }).fill("");
  await page
    .getByRole("combobox", { name: "Container state", exact: true })
    .selectOption("exited");
  await expect(
    page.getByText("500 matching · 999 total", { exact: false }),
  ).toBeVisible();
  await table.getByRole("button", { name: /^Name/ }).click();
  await expect(table.locator("th").first()).toHaveAttribute(
    "aria-sort",
    "descending",
  );
  await page.locator(".container-scroll").evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect(
    table.getByRole("button", { name: "workload-0000", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

test("live host switch cannot publish a late snapshot with identical container names and IDs", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const ids = [`h_${"1".repeat(32)}`, `h_${"2".repeat(32)}`];
    const tokens = [`s_${"a".repeat(32)}`, `s_${"b".repeat(32)}`];
    let current = 0;
    let finishOld: (() => void) | undefined;
    const options = { executable: null, context: null, sudo: false };
    const hosts = ids.map((id, index) => ({
      id,
      alias: `fixture-${index}`,
      displayName: "Identical host name",
      group: "test",
      labels: [],
      favorite: false,
      readOnly: true,
      ssh: {
        alias: `fixture-${index}`,
        configPath: "/fixture/config",
        useDefaultConfig: false,
      },
      docker: options,
    }));
    const saved = {
      writable: true,
      notice: null,
      preferences: {
        schemaVersion: 3,
        revision: 0,
        theme: "system",
        hosts,
        selectedHostId: ids[0],
        trustedConfigPath: null,
        sshExecutableOverride: null,
      },
    };
    Reflect.set(window, "switchFixtureHost", () => {
      current = 1;
    });
    Reflect.set(window, "finishOldFixtureRead", () => finishOld?.());
    Reflect.set(window, "isTauri", true);
    Reflect.set(window, "__TAURI_INTERNALS__", {
      async invoke(
        command: string,
        args?: {
          request?: {
            selection?: { hostId: string; selectionGeneration: number };
            scope?: {
              selection: { hostId: string; selectionGeneration: number };
              sessionId: string;
              sessionGeneration: number;
              daemonId: string;
            };
          };
        },
      ) {
        if (command === "app_version") return { version: "fixture-020" };
        if (command === "get_preferences") return saved;
        if (command === "get_workspace_mode")
          return { mode: "live", scenario: null, scope: null, host: null };
        if (command === "get_host_inventory")
          return {
            mode: "live",
            saved,
            connection: {
              hostId: ids[current],
              token: {
                sessionId: tokens[current],
                sessionGeneration: current + 1,
              },
              selection: hosts[current]?.ssh,
              state: "ready",
              durations: [],
              diagnostic: null,
              hasJump: false,
              effective: null,
              transportMode: "multiplexed",
              dockerOptions: options,
              docker: {
                status: "ready",
                context: "default",
                endpoint: "unix:///fixture/docker.sock",
                endpointKind: "unix",
                clientVersion: "fixture",
                serverVersion: "fixture",
                daemonId: "fixture-daemon",
                os: "linux",
                rootless: false,
                compose: "absent",
                composeVersion: null,
                sudo: false,
              },
            },
          };
        if (command === "connect_host")
          return {
            scope: {
              selection: args?.request?.selection,
              sessionId: tokens[current],
              sessionGeneration: current + 1,
              daemonId: "fixture-daemon",
            },
            capabilities: {
              docker: true,
              compose: false,
              management: false,
              terminal: false,
            },
          };
        if (command === "list_containers") {
          const scope = args?.request?.scope;
          if (!scope) throw { code: "invalid_id" };
          const response = {
            scope,
            containers: [
              {
                scope,
                id: "a".repeat(64),
                name: "Identical container",
                image:
                  scope.selection.hostId === ids[0]
                    ? "old-host-image"
                    : "current-host-image",
                state: "running",
                status: "fixture",
                health: null,
                ports: [],
                compose: null,
                cli: null,
              },
            ],
          };
          if (scope.selection.hostId === ids[0]) {
            Reflect.set(window, "oldFixtureReadStarted", true);
            return new Promise((resolve) => {
              finishOld = () => resolve(response);
            });
          }
          return response;
        }
        throw { code: "feature_unavailable" };
      },
    });
  });
  await page.goto("/");
  await expect
    .poll(() =>
      page.evaluate(() => Reflect.get(window, "oldFixtureReadStarted")),
    )
    .toBe(true);
  await page.evaluate(() =>
    (Reflect.get(window, "switchFixtureHost") as () => void)(),
  );
  await expect(
    page.getByRole("cell", { name: "current-host-image", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Identical container", exact: true })
    .click();
  await page.evaluate(() =>
    (Reflect.get(window, "finishOldFixtureRead") as () => void)(),
  );
  await page.getByRole("button", { name: "Refresh containers" }).click();
  await expect(
    page.getByRole("button", { name: "Refresh containers" }),
  ).toBeEnabled();
  await expect(
    page.getByRole("complementary", { name: "Resource details" }),
  ).toContainText("current-host-image");
  await expect(page.getByText("old-host-image", { exact: true })).toHaveCount(
    0,
  );
  await expect(page.getByRole("status", { name: "Demo mode" })).toHaveCount(0);
});
