import { chromium } from "@playwright/test";
import { strict as assert } from "node:assert";
import { mkdirSync, readFileSync } from "node:fs";
const output = process.env.DOCS_SCREENSHOTS || "test-results/docs";
mkdirSync(output, { recursive: true });
const navigation = JSON.parse(
  readFileSync(
    new URL("../../docs/_data/navigation.yml", import.meta.url),
    "utf8",
  ),
);
const pageCount = navigation.reduce(
  (count, group) => count + group.pages.length,
  0,
);
const browser = await chromium.launch({
  headless: true,
  args: ["--no-sandbox"],
});
const base = process.env.DOCS_URL || "http://127.0.0.1:4178/container-desk/";
const errors = [];
const results = [];
for (const width of [1440, 768, 390]) {
  const page = await browser.newPage({
    viewport: { width, height: 960 },
    colorScheme: "light",
  });
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("response", (r) => {
    if (r.status() >= 400) errors.push(`${r.status()} ${r.url()}`);
  });
  await page.goto(base);
  await page.waitForSelector(".js-ready");
  assert.equal(await page.locator("h1").count(), 1);
  assert.equal(await page.locator(".nav-group a").count(), pageCount);
  assert.ok(
    await page
      .locator("article img")
      .evaluateAll((imgs) =>
        imgs.every((i) => i.complete && i.naturalWidth > 0),
      ),
  );
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth + 1,
    ),
  );
  await page.screenshot({ path: `${output}/home-${width}-light.png` });
  await page.getByRole("button", { name: "Switch to dark theme" }).click();
  await page.reload();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  if (width < 760) {
    await page.locator("#menu").click();
    assert.equal(
      await page.locator("#menu").getAttribute("aria-expanded"),
      "true",
    );
  }
  await page.getByLabel("Find a guide").fill("SSH");
  const matches = await page.locator(".nav-group li:visible").count();
  assert.ok(matches > 0 && matches < 20);
  await page.getByLabel("Find a guide").fill("no-such-guide-xyz");
  assert.equal(
    await page.locator("#search-status").innerText(),
    "0 matching guides",
  );
  await page.getByLabel("Find a guide").fill("");
  if (width < 760) {
    await page.keyboard.press("Escape");
    assert.equal(
      await page.locator("#menu").getAttribute("aria-expanded"),
      "false",
    );
  }
  await page.goto(base + "user-guide.html");
  await page.waitForSelector(".js-ready");
  assert.ok(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth + 1,
    ),
  );
  await page.screenshot({ path: `${output}/guide-${width}-dark.png` });
  assert.ok((await page.locator("#toc a").count()) > 0);
  await page.goto(base + "container-statistics.html");
  assert.ok((await page.locator("article").innerText()).includes("{{json .}}"));
  await page.goto(base + "signing.html");
  assert.ok(
    (await page.locator("article").innerText()).includes(
      "${{ secrets.APPLE_CERTIFICATE }}",
    ),
  );
  results.push({
    width,
    themePersistence: true,
    search: true,
    noOverflow: true,
    literalTemplates: true,
  });
  await page.close();
}
const nojs = await browser.newPage({
  javaScriptEnabled: false,
  viewport: { width: 390, height: 844 },
});
await nojs.goto(base);
assert.ok(await nojs.locator(".sidebar").isVisible());
assert.ok(await nojs.locator("article").isVisible());
await nojs.close();
await browser.close();
assert.deepEqual(errors, []);
console.log(
  JSON.stringify({ base, results, noJavaScript: true, errors }, null, 2),
);
