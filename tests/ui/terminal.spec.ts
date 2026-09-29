import { expect, test, type Page } from "@playwright/test";
async function connect(page: Page) {
  await page.getByRole("tab", { name: "Terminal", exact: true }).click();
  await page
    .getByRole("button", { name: "Enable management for terminal" })
    .click();
  await page
    .getByRole("button", { name: "Enable terminal access", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Open terminal", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Confirm terminal access" }),
  ).toContainText("user 1000:1000");
  await page
    .getByRole("button", { name: "Confirm terminal", exact: true })
    .click();
  await expect(page.locator(".terminal-status")).toContainText("Connected");
}
async function paste(page: Page, text: string) {
  const target = page.locator(".xterm-helper-textarea");
  await target.focus();
  await target.evaluate((node, value) => {
    const data = new DataTransfer();
    data.setData("text/plain", value);
    node.dispatchEvent(
      new ClipboardEvent("paste", {
        clipboardData: data,
        bubbles: true,
        cancelable: true,
      }),
    );
  }, text);
}
test("explicit terminal, safe output, resize and multiline paste confirmation", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "clipboard", {
      value: {
        writeText: async () => {
          (window as unknown as { clipboardWrites: number }).clipboardWrites++;
        },
      },
    });
    (window as unknown as { clipboardWrites: number }).clipboardWrites = 0;
  });
  await page.goto("/tests/ui/fixture.html?state=terminal");
  await connect(page);
  await expect(page.locator(".terminal-identity")).toContainText(
    "Fixture host 1",
  );
  await expect(
    page.locator(".terminal-panel img, .terminal-panel svg, .terminal-panel a"),
  ).toHaveCount(0);
  await expect(page).toHaveTitle("ContainerDesk UI fixture — synthetic data");
  expect(
    await page.evaluate(
      () => (window as unknown as { clipboardWrites: number }).clipboardWrites,
    ),
  ).toBe(0);
  await expect(page.getByLabel("Fixture terminal size")).toHaveText(/\d+x\d+/);
  const old = await page.getByLabel("Fixture terminal size").textContent();
  await page.setViewportSize({ width: 1100, height: 900 });
  await expect(page.getByLabel("Fixture terminal size")).not.toHaveText(
    old ?? "",
  );
  await paste(page, "echo first\necho second\n");
  await expect(
    page.getByRole("dialog", { name: "Confirm multiline paste" }),
  ).toContainText("echo second");
  await expect(page.getByLabel("Fixture terminal input")).toBeEmpty();
  await page.getByRole("button", { name: "Cancel paste", exact: true }).click();
  await expect(page.getByLabel("Fixture terminal input")).toBeEmpty();
  await paste(page, "echo first\necho second\n");
  await page.getByRole("button", { name: "Send multiline paste" }).click();
  await expect(page.getByLabel("Fixture terminal input")).toContainText(
    "echo second",
  );
  await page
    .getByRole("button", { name: "Disconnect terminal", exact: true })
    .click();
  await expect(page.getByLabel("Fixture terminal closes")).toHaveText("1");
});
test("close under output, host switch, inactive input and revocation never reopen", async ({
  page,
}) => {
  await page.goto("/tests/ui/fixture.html?state=terminal");
  await connect(page);
  await page.getByRole("button", { name: "Fixture output flood" }).click();
  await page.getByRole("tab", { name: "Logs", exact: true }).click();
  await expect(page.getByLabel("Fixture terminal closes")).toHaveText("1");
  await page.keyboard.type("must-not-go-to-old-terminal");
  await expect(page.getByLabel("Fixture terminal input")).toBeEmpty();
  await page.getByRole("button", { name: "Fixture switch host" }).click();
  await connect(page);
  await expect(page.locator(".terminal-identity")).toContainText(
    "Fixture host 2",
  );
  await page.locator(".xterm-helper-textarea").focus();
  await page.keyboard.type("owned-second");
  await expect(page.getByLabel("Fixture terminal input")).toContainText(
    `h_${"2".repeat(32)}:`,
  );
  await page.getByRole("button", { name: "Fixture revoke permission" }).click();
  await expect(page.locator(".terminal-status")).toContainText("not permitted");
  await expect(page.getByLabel("Fixture terminal closes")).toHaveText("2");
  await expect(page.getByLabel("Fixture terminal opens")).toHaveText("2");
  expect(
    await page.evaluate(() => JSON.stringify({ ...localStorage })),
  ).not.toContain("synthetic-040-stream");
});
