import { test, expect, Page } from "@playwright/test";

const BASE = "http://localhost:3000";

// The wgpu canvas renderer logs device-limit errors in headless browsers;
// they're unrelated to routing.
const IGNORED = [/WebGPU/i, /wgpu/i, /webgl/i, /GPU/];

function collectConsoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error" && !IGNORED.some((re) => re.test(msg.text()))) {
      errors.push(msg.text());
    }
  });
  page.on("pageerror", (err) => {
    errors.push(String(err));
  });
  return errors;
}

test("islands router navigation between plain pages works (control)", async ({
  page,
}) => {
  const errors = collectConsoleErrors(page);

  await page.goto(`${BASE}/blog`);
  await expect(page.locator("h1", { hasText: "blog" })).toBeVisible();

  await page.click('a[href="/"]');
  await expect(page).toHaveURL(`${BASE}/`);
  await expect(page.locator("h1", { hasText: "sean aye" })).toBeVisible();

  expect(errors).toEqual([]);
});

test("navigating from the share page to home replaces the html", async ({
  page,
}) => {
  const errors = collectConsoleErrors(page);

  // an invalid token still exercises the Suspense streaming path: the page
  // streams the shell, then the "invalid or unavailable" message.
  await page.goto(`${BASE}/photo/not-a-real-token`);
  await expect(page.locator("#share-grid")).toBeAttached();
  await expect(
    page.locator("text=This share link is invalid or unavailable."),
  ).toBeVisible();
  // once the grid streams in, the :has() rule hides the loading indicator
  await expect(page.locator("#share-loading")).toBeHidden();

  // navigate to the home page via the navbar
  await page.click('a[href="/"]');
  await expect(page).toHaveURL(`${BASE}/`);

  // the home page content must replace the share page content
  await expect(page.locator("h1", { hasText: "sean aye" })).toBeVisible();
  await expect(page.locator("#share-grid")).not.toBeAttached();

  expect(errors).toEqual([]);
});
