// The generated app in a browser: signs in and visits every kit feature, failing on the first one that is
// missing. Screenshots of each step go to <out>.
//
//   node scripts/template-smoke.cjs <base url> <user> <password> <out dir>
//
// Needs Playwright (preinstalled in the kit's CI container image, or `npm i playwright`).
const { chromium } = require('playwright');
const fs = require('node:fs');

(async () => {

const [base, user, password, out] = process.argv.slice(2);
fs.mkdirSync(out, { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 860 } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
let n = 0;
async function step(name, fn) {
  try {
    await fn();
    await page.screenshot({ path: `${out}/${String(++n).padStart(2, '0')}-${name}.png` });
    console.log(`PASS ${name}`);
  } catch (e) {
    await page.screenshot({ path: `${out}/${String(++n).padStart(2, '0')}-${name}-FAILED.png` }).catch(() => {});
    console.log(`FAIL ${name}: ${e.message}`);
    await browser.close();
    process.exit(1);
  }
}
const see = (sel, timeout = 15000) => page.locator(sel).first().waitFor({ state: 'visible', timeout });

await step('sign-in-page', async () => {
  await page.goto(base);
  await see('.auth-card');
});
await step('wrong-password-is-said-plainly', async () => {
  await page.fill('input[name=username]', user);
  await page.fill('input[name=password]', 'not the password');
  await page.click('button[type=submit]');
  await page.locator('#auth-err', { hasText: 'Invalid username or password' }).waitFor();
});
await step('whats-new-popup-after-signing-in', async () => {
  await page.fill('input[name=password]', password);
  await page.click('button[type=submit]');
  await see('.shell');
  await see('.modal-bg .modal');
});
await step('home', async () => {
  await page.locator('.modal-bg [data-close]').first().click();
  await see('.hero');
});
await step('settings-form-from-the-schema', async () => {
  await page.goto(`${base}/#/settings?s=general`);
  await page.locator('text=App name').first().waitFor();
});
await step('diagnostics', async () => {
  await page.goto(`${base}/#/settings?s=diagnostics`);
  await page.locator('#diag', { hasText: 'Data folder' }).waitFor({ timeout: 20000 });
});
await step('whats-new-page', async () => {
  await page.goto(`${base}/#/settings?s=whats-new`);
  await page.locator('#view', { hasText: '0.1.0' }).first().waitFor();
});
await step('bug-report-sheet', async () => {
  await page.goto(`${base}/#/bug-reports`);
  await see('#bugs-list');
  await page.click('#bugs-new');
  await see('.modal-bg .modal');
});
await step('details-dialog-with-copy', async () => {
  await page.keyboard.press('Escape');
  await page.evaluate(() => {
    const b = document.createElement('button');
    b.id = 'details-probe';
    b.textContent = 'details';
    document.querySelector('#view').append(b);
  });
  // the kit opens the details dialog for any [data-details] element; set the attributes the kit uses
  await page.evaluate(() => {
    const b = document.querySelector('#details-probe');
    b.setAttribute('data-details', 'Something went wrong\nwith every detail');
    b.setAttribute('data-details-title', 'Details');
  });
  await page.click('#details-probe');
  await see('.modal-bg .modal');
});
await step('sign-out-asks-first', async () => {
  await page.keyboard.press('Escape');
  await page.click('#logout-btn');
  await page.locator('.modal-bg .modal', { hasText: 'Sign out?' }).waitFor();
  await page.locator('.modal-bg button[data-close]', { hasText: 'Cancel' }).click();
  await page.locator('.modal-bg').waitFor({ state: 'detached' });
  await see('.shell');
});
await step('signed-out-after-ok', async () => {
  await page.click('#logout-btn');
  await page.locator('.modal-bg button[data-ok]', { hasText: 'Sign out' }).click();
  await see('.auth-card');
});
await step('opens-with-no-connection', async () => {
  // the app-shell worker (/sw.js) keeps this build's page and files: a reload with no network still draws the app
  const ready = await page.evaluate(() => navigator.serviceWorker.ready.then((r) => !!r.active));
  if (!ready) throw new Error('the service worker did not start');
  const kept = await page.evaluate(() => caches.keys());
  if (!kept.some((k) => k.includes('-shell-'))) throw new Error(`no shell cache: ${kept.join(', ')}`);
  await page.context().setOffline(true);
  try {
    await page.reload();
    await see('.auth-card, .shell');
  } finally {
    await page.context().setOffline(false);
  }
});
if (errors.length) {
  console.log(`FAIL page errors:\n${errors.join('\n')}`);
  await browser.close();
  process.exit(1);
}
await browser.close();
console.log('every kit feature showed');
})();
