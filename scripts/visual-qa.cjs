// Visual QA: every page of a web app at a phone (390×844), a tablet (820×1180) and a desktop (1440×900), with a
// full-page screenshot of each and two checks that fail the run:
//   - horizontal overflow: the page scrolls sideways, or an element reaches past the screen's edge (content inside
//     a scroller that is meant to scroll sideways, such as a carousel, doesn't count);
//   - text overlap: two runs of text in the page's flow cover each other (fixed and sticky bars, toasts and
//     banners float over the page by design and are left out; text cut off by its box counts only where it shows).
// The pages are drawn with reduced motion, so the screenshots are still and the reduced-motion rules are proved.
//
//   node scripts/visual-qa.cjs <base url> <out dir> [pages.json]
//
// pages.json: [{ "name": "home", "path": "#/", "wait": ".k-page", "sheet": "#open-sheet" }, …] (`sheet`: a button
// that opens a sheet, screenshotted too). Without it, the kit-leptos demo's pages. The page must set
// <body data-kit-check="pass|fail"> when it checks itself (the demo does); "fail" fails the run.
// Writes <out>/<viewport>/<page>.png, <out>/report.json and <out>/report.md. Needs Playwright.
const { chromium } = require('playwright');
const fs = require('node:fs');
const path = require('node:path');

const VIEWPORTS = [
  { name: 'phone', width: 390, height: 844, mobile: true },
  { name: 'tablet', width: 820, height: 1180, mobile: false },
  { name: 'desktop', width: 1440, height: 900, mobile: false },
];
const DEMO_PAGES = [
  { name: 'home', path: '#/' },
  { name: 'browse', path: '#/browse' },
  { name: 'item', path: '#/item/3' },
  { name: 'stats', path: '#/stats' },
  { name: 'states', path: '#/states', sheet: '#open-sheet' },
  { name: 'kit', path: '#/kit' },
];

// Runs in the page: what overflows sideways and which texts overlap.
function inspect() {
  const vw = document.documentElement.clientWidth;
  const style = (el) => getComputedStyle(el);
  const clips = (el) => { const s = style(el); return s.overflowX !== 'visible' || s.overflowY !== 'visible'; };
  const describe = (el) => {
    let d = el.tagName.toLowerCase();
    if (el.id) d += `#${el.id}`;
    const c = typeof el.className === 'string' ? el.className.trim().split(/\s+/).filter(Boolean).slice(0, 3) : [];
    if (c.length) d += `.${c.join('.')}`;
    return d;
  };
  const hidden = (el) => {
    for (let e = el; e && e !== document.documentElement; e = e.parentElement) {
      const s = style(e);
      if (s.display === 'none' || s.visibility === 'hidden' || Number(s.opacity) === 0) return true;
      // the body of a closed <details> keeps its layout styles but isn't drawn
      const up = e.parentElement;
      if (up && up.tagName === 'DETAILS' && !up.open && e.tagName !== 'SUMMARY') return true;
    }
    return false;
  };
  const floating = (el) => {
    for (let e = el; e && e !== document.body; e = e.parentElement) {
      const p = style(e).position;
      if (p === 'fixed' || p === 'sticky') return true;
    }
    return false;
  };
  // the part of `r` that shows: cut by every ancestor (from `from` up) that clips its content
  const visiblePart = (r, from) => {
    let { left, top, right, bottom } = r;
    for (let e = from; e && e !== document.body && e !== document.documentElement; e = e.parentElement) {
      if (!clips(e)) continue;
      const c = e.getBoundingClientRect();
      left = Math.max(left, c.left); top = Math.max(top, c.top); right = Math.min(right, c.right); bottom = Math.min(bottom, c.bottom);
    }
    return right - left > 0.5 && bottom - top > 0.5 ? { left, top, right, bottom } : null;
  };

  const overflow = [];
  const doc = document.documentElement.scrollWidth;
  if (doc > vw + 1) overflow.push(`the page is ${doc}px wide on a ${vw}px screen`);
  for (const el of document.body.querySelectorAll('*')) {
    if (el.closest('svg') && el.tagName.toLowerCase() !== 'svg') continue;
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) continue;
    const v = visiblePart(r, el.parentElement);
    if (!v) continue;
    if (v.right > vw + 1 || v.left < -1) {
      if (hidden(el)) continue;
      overflow.push(`${describe(el)} reaches ${Math.round(v.left)}…${Math.round(v.right)}px on a ${vw}px screen`);
    }
  }

  const runs = [];
  const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let n = walk.nextNode(); n; n = walk.nextNode()) {
    const text = n.textContent.trim();
    const el = n.parentElement;
    if (!text || !el || el.closest('svg, script, style, noscript') || hidden(el) || floating(el)) continue;
    const range = document.createRange();
    range.selectNodeContents(n);
    for (const r of range.getClientRects()) {
      const v = visiblePart(r, el);
      if (v) runs.push({ node: n, el, text: text.slice(0, 40), ...v });
    }
  }
  const overlap = [];
  for (let i = 0; i < runs.length; i++) {
    for (let j = i + 1; j < runs.length; j++) {
      const a = runs[i], b = runs[j];
      if (a.node === b.node) continue;
      const w = Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (w > 2 && h > 2) overlap.push(`"${a.text}" (${describe(a.el)}) and "${b.text}" (${describe(b.el)}) overlap by ${Math.round(w)}×${Math.round(h)}px`);
    }
  }
  return { overflow: [...new Set(overflow)].slice(0, 20), overlap: overlap.slice(0, 20) };
}

(async () => {
  const [base, out, pagesFile] = process.argv.slice(2);
  if (!base || !out) {
    console.error('usage: node scripts/visual-qa.cjs <base url> <out dir> [pages.json]');
    process.exit(2);
  }
  const pages = pagesFile ? JSON.parse(fs.readFileSync(pagesFile, 'utf8')) : DEMO_PAGES;
  const browser = await chromium.launch();
  const results = [];
  const errors = [];
  for (const vp of VIEWPORTS) {
    fs.mkdirSync(path.join(out, vp.name), { recursive: true });
    const ctx = await browser.newContext({ viewport: { width: vp.width, height: vp.height }, deviceScaleFactor: 1, isMobile: vp.mobile, hasTouch: vp.mobile, reducedMotion: 'reduce' });
    const page = await ctx.newPage();
    page.on('pageerror', (e) => errors.push(`${vp.name}: ${e}`));
    await page.goto(base);
    // the page's own check, when it has one
    const check = await page.waitForFunction(() => document.body.dataset.kitCheck, null, { timeout: 30000 }).then((h) => h.jsonValue()).catch(() => 'none');
    if (check === 'fail') errors.push(`${vp.name}: the page's own check failed`);
    for (const p of pages) {
      await page.evaluate((h) => { location.hash = h; }, p.path);
      await page.locator(p.wait ?? '.k-page').first().waitFor({ state: 'visible', timeout: 15000 });
      // what floats over the page by design (toasts, banners) is left out of the pictures
      await page.evaluate(() => {
        document.querySelectorAll('#toasts > *, body > #offline-banner, body > #conn-banner').forEach((e) => e.remove());
        window.scrollTo(0, 0);
      });
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(250);
      const shots = [{ name: p.name, open: null }];
      if (p.sheet) shots.push({ name: `${p.name}-sheet`, open: p.sheet });
      for (const s of shots) {
        if (s.open) {
          await page.click(s.open);
          await page.locator('.k-sheet, .modal').first().waitFor({ state: 'visible' });
          await page.waitForTimeout(150);
        }
        const found = await page.evaluate(inspect);
        const file = path.join(vp.name, `${s.name}.png`);
        // a full-page picture draws fixed bars where the screen was: pin the tab bar to the page's foot for it
        const pin = s.open ? null : await page.addStyleTag({ content: '.k-shell { position: relative; } .k-tabbar { position: absolute; }' });
        await page.screenshot({ path: path.join(out, file), fullPage: !s.open });
        if (pin) await pin.evaluate((e) => e.remove());
        const ok = !found.overflow.length && !found.overlap.length;
        results.push({ viewport: vp.name, size: `${vp.width}×${vp.height}`, page: s.name, file, ok, ...found });
        console.log(`${ok ? 'PASS' : 'FAIL'} ${vp.name} ${s.name}`);
        for (const o of found.overflow) console.log(`  overflow: ${o}`);
        for (const o of found.overlap) console.log(`  overlap: ${o}`);
        if (s.open) {
          await page.keyboard.press('Escape');
          await page.locator('.k-sheet-bg, .modal-bg').first().waitFor({ state: 'detached' }).catch(() => {});
        }
      }
    }
    await ctx.close();
  }
  await browser.close();
  const failed = results.filter((r) => !r.ok);
  fs.writeFileSync(path.join(out, 'report.json'), JSON.stringify({ results, errors }, null, 2));
  let md = `| Page | ${VIEWPORTS.map((v) => `${v.name} ${v.width}×${v.height}`).join(' | ')} |\n|---|${VIEWPORTS.map(() => '---').join('|')}|\n`;
  for (const name of [...new Set(results.map((r) => r.page))]) {
    md += `| ${name} | ${VIEWPORTS.map((v) => { const r = results.find((x) => x.page === name && x.viewport === v.name); return r ? (r.ok ? 'pass' : 'FAIL') : ''; }).join(' | ')} |\n`;
  }
  for (const r of failed) {
    md += `\n**${r.viewport} ${r.page}**\n${[...r.overflow.map((o) => `- overflow: ${o}`), ...r.overlap.map((o) => `- overlap: ${o}`)].join('\n')}\n`;
  }
  if (errors.length) md += `\n**Page errors**\n${errors.map((e) => `- ${e}`).join('\n')}\n`;
  fs.writeFileSync(path.join(out, 'report.md'), md);
  if (failed.length || errors.length) {
    console.log(`\n${failed.length} page(s) failed, ${errors.length} page error(s): see ${path.join(out, 'report.md')}`);
    process.exit(1);
  }
  console.log(`\nevery page passed at every size (${results.length} screenshots)`);
})();
