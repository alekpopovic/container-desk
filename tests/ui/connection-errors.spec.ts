import { expect, test } from "@playwright/test";

for (const [code, advice] of [
  ["unknown_host_key", "Verify its fingerprint independently in your terminal"],
  ["changed_host_key", "Stop and verify the change with its administrator"],
  [
    "authentication_failed",
    "key may not be loaded in the desktop session agent",
  ],
  ["timed_out", "Retry explicitly after checking access"],
]) {
  test(`${code} stays actionable and never automatically retries`, async ({
    page,
  }) => {
    await page.clock.install();
    await page.goto(
      `/tests/ui/fixture.html?state=connection-errors&failure=${code}`,
    );
    const panel = page.getByRole("region", { name: "SSH connection session" });
    await expect(page.getByLabel("Connection starts")).toHaveText("0");
    await panel.getByRole("button", { name: "Connect selected host" }).click();
    await expect(panel.getByRole("status").first()).toHaveText(
      "Connection error",
    );
    await expect(panel).toContainText(advice!);
    await expect(panel).not.toContainText("Ready");
    await page.clock.fastForward(60_000);
    await expect(page.getByLabel("Connection starts")).toHaveText("1");
    await expect(
      panel.getByLabel("Remote Docker context (optional)"),
    ).toBeEnabled();
    await panel.getByRole("button", { name: "Connect selected host" }).click();
    await expect(page.getByLabel("Connection starts")).toHaveText("2");
  });
}
