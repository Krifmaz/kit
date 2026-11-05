// The kit's app-shell service worker (kit-server-health `worker_router`). The server serves it at /sw.js with
// `self.KIT_SW = {app, build, api}` written above this line, so every new web build is a new worker:
// the browser installs it next to the old one, it keeps the new build's shell, waits, and takes over when the
// page says so (kit-rules `update::Flow`: `{type: 'skip-waiting'}`; `{type: 'version?'}` is answered with
// `{type: '<app>:version', cache: '<app>-shell-<build>'}`).
//
// What it keeps: the page (`/`) and the files it names (scripts, styles, pictures), in `<app>-shell-<build>`.
// Pages are asked of the network first and come from the cache only when the network fails, so the app opens
// with no connection. Files whose names carry a content hash come from the cache first. The API (`api`, `/api/`)
// is never touched: what an app keeps of its data is its own.
'use strict';

const CFG = self.KIT_SW || {};
const APP = String(CFG.app || 'app').toLowerCase();
const BUILD = String(CFG.build || 'dev');
const API = String(CFG.api || '/api/');
const SHELL = `${APP}-shell-`;
const CACHE = SHELL + BUILD;

/** The same-origin files a page names in `src` and `href` (not the API, not other sites). */
function filesOf(html) {
  const out = new Set();
  const re = /\s(?:src|href)\s*=\s*["']([^"'#?]+)/gi;
  let m;
  while ((m = re.exec(html))) {
    let u;
    try { u = new URL(m[1], self.location.origin); } catch (_) { continue; }
    if (u.origin === self.location.origin && !u.pathname.startsWith(API) && u.pathname !== '/sw.js') out.add(u.pathname);
  }
  return [...out];
}

/** A name whose file never changes: `<name>-<12+ hex digits>[_bg].<ext>` (trunk's file hashes). */
function hashed(path) {
  return /-[0-9a-f]{12,}(_bg)?\.[a-z0-9]+$/i.test(path);
}

self.addEventListener('install', (e) => {
  e.waitUntil((async () => {
    const cache = await caches.open(CACHE);
    const page = await fetch('/', { cache: 'no-store', credentials: 'same-origin' });
    if (!page.ok) throw new Error(`the page answered ${page.status}`);
    const html = await page.clone().text();
    await cache.put('/', page);
    // a file that can't be had now is fetched again when the page asks for it
    await Promise.all(filesOf(html).map((f) => cache.add(f).catch(() => {})));
  })());
});

self.addEventListener('activate', (e) => {
  e.waitUntil((async () => {
    for (const k of await caches.keys()) if (k.startsWith(SHELL) && k !== CACHE) await caches.delete(k);
    await self.clients.claim();
  })());
});

self.addEventListener('message', (e) => {
  const d = e.data || {};
  if (d.type === 'skip-waiting') self.skipWaiting();
  else if (d.type === 'version?' && e.source) e.source.postMessage({ type: `${APP}:version`, cache: CACHE });
});

self.addEventListener('fetch', (e) => {
  const req = e.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin || url.pathname.startsWith(API) || url.pathname === '/sw.js') return;
  if (req.mode === 'navigate') {
    // the newest page when the server answers; the kept one when it doesn't
    e.respondWith(fetch(req).catch(async () => (await caches.match('/', { cacheName: CACHE })) || Response.error()));
    return;
  }
  if (hashed(url.pathname)) {
    e.respondWith((async () => {
      const kept = await caches.match(req, { cacheName: CACHE });
      if (kept) return kept;
      const r = await fetch(req);
      if (r.ok) (await caches.open(CACHE)).put(req, r.clone()).catch(() => {});
      return r;
    })());
    return;
  }
  // anything else (icons, the manifest): the network, else the kept copy
  e.respondWith(fetch(req).catch(async () => (await caches.match(req, { cacheName: CACHE })) || Response.error()));
});
