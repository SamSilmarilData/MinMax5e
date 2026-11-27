---
status: proposed
---

# The web build is hosted on Cloudflare Workers at a free workers.dev URL, behind per-friend invite links

**Host: Cloudflare Workers static assets, free plan, at `https://minmax5e.<account-subdomain>.workers.dev`.**

- **Why Workers rather than Pages:** Cloudflare's Pages docs now open with "Start new projects with Workers" [verified 2026-11-27].
- **Free:** static asset requests are free and unlimited. Worker invocations count against 100,000 requests a day on the Workers Free plan [verified, Cloudflare docs].
- **Unique hostname:** the URL is `<worker-name>.<account-subdomain>.workers.dev`. The account subdomain is chosen once in the dashboard. `workers.dev` is on the Public Suffix List [verified], so the hostname is its own site for cookies and storage.
- **No custom domain**, by owner decision.

**Gate: invite links, so friends do nothing extra.** The friend taps a link once, and from then on the URL just opens.

1. **Issuing:** the owner gives each friend a bearer link, `/join/<token>`. Tokens are random, 128-bit or more, one per friend, and stored as Worker **secrets** (`wrangler secret put`), never in the repo.
2. **Joining:** the Worker checks the token. It then sets `mm_invite`, an HMAC-SHA-256-signed `{friend_id, generation, issued_at}` cookie (`HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=400 days`), and redirects to `/`.
3. **Every visit:** on gated routes, the Worker verifies the signature and checks that `generation` matches that friend's current generation. It also re-issues the cookie, so the 400-day expiry rolls forward. Chrome caps cookie lifetime at 400 days ([Chrome](https://developer.chrome.com/blog/cookie-max-age-expires)).
4. **No valid cookie:** the visitor gets a small page saying "ask whoever sent you the link", with `noindex`, plus `X-Robots-Tag: noindex` on every response.
5. **Typed invite codes, for in-app browsers.** A link tapped inside Discord, Instagram, or WhatsApp often opens an in-app browser. On iOS these don't share cookies with Safari: `SFSafariViewController` stopped sharing website data in iOS 11, and `WKWebView` keeps its own cookie jar. On Android, WebView is isolated, but Custom Tabs share Chrome's cookies (research 12 §2). So:
   - Each invite also has a **short code**: 8 characters from an unambiguous alphabet (no `0/O`, `1/l/I`), mapped server-side to the same token.
   - The gate page offers a code box.
   - The page detects the common in-app browsers from their user agents and shows "Open in Safari/Chrome" instructions.
   - The owner sends the link and the code together.
   - Codes are rate-limited per IP in the Worker. They're brute-force resistant at about 40 bits, and revocable like tokens.

Why the cookie survives in Safari [verified at WebKit]:

- ITP's 7-day cap applies to *script-writable* storage and client-side cookies ([WebKit, 2020](https://webkit.org/blog/10218/full-third-party-cookie-blocking-and-more/)).
- WebKit itself recommends a server-set `Secure`, `HttpOnly` cookie for durable first-party sessions.
- WebKit's server-set-cookie cap targets third-party CNAME or IP cloaking ([WebKit](https://webkit.org/blog/11338/cname-cloaking-and-web-privacy/)). Here the cookie comes from the site's own host.
- Cookies aren't subject to WebKit's storage-eviction policy ([WebKit, 2023](https://webkit.org/blog/14403/updates-to-storage-policy/)).
- Spike W4 still checks 8+ days without a visit on iOS.

**Configuration:**

- **Only the entry points are gated.** `assets.run_worker_first = ["/", "/index.html", "/join/*"]`. The `.wasm` and `.js` files, the service worker, the manifest, and the icons are served directly.
- **Why that's safe:** those files are exactly the public repo's code (ADR-0028), so gating them protects nothing. Leaving them ungated keeps them free of Worker invocations and lets `_headers` rules apply. Custom `_headers` are not applied to Worker-generated responses. The browser also fetches the web app manifest *without* cookies unless `crossorigin="use-credentials"` is set (MDN), so a gated manifest would break install.
- **Caching:** hashed asset filenames get `Cache-Control: public, max-age=31536000, immutable` in `_headers`. Entry HTML stays `no-cache`.
- **Compression:** Cloudflare's default compression list includes `application/wasm`. The free plan defaults to Zstandard, with Brotli and Gzip as fallbacks [verified, Cloudflare docs]. Confirm on the wire in W1.
- **Fail closed:** the route's failure mode is set to **fail closed**. If the free daily allowance were ever exhausted, gated routes return Cloudflare's 1027 error instead of bypassing the gate. A visit costs about one invocation, so a handful of friends never comes close.

**Revoking:**

- One friend: bump their generation, or delete their token.
- Everyone: rotate the HMAC key.
- What a revoked friend keeps: an installed copy already in their service-worker cache keeps working offline until it updates. It contains no rules data that isn't fetched from the public mirror anyway. Accepted.

**Leaked links:** an invite link is a bearer credential. If a friend posts it publicly, revoke that token.

**Link previews:** the invite page and the fallback page carry Open Graph tags (title, description, image, all our own content), so a link pasted into iMessage, Discord, or WhatsApp shows a card.

**Deploy:**

- GitHub Actions builds with Trunk and deploys with `cloudflare/wrangler-action` (v4.1.3, 2026-09-24).
- A scoped API token is kept in repository secrets.
- **CI never fetches the corpus.** It fails the deploy if `dist/` contains any file outside the expected set (the `.wasm`, `.js`, `.html`, `.css`, icon and manifest files), or any `{@` markup (ADR-0028).
- The gate script is in the repo, and its secrets aren't.

## Considered Options

- **Public, ungated (GitHub Pages or `workers.dev`).** Rejected: anyone, including crawlers, could use a 5e.tools-powered app at a public URL. ADR-0026 treats that as going public. The gate costs friends nothing.
- **Cloudflare Access with email one-time PIN** (free up to 50 users). Rejected: email, inbox, and a 6-digit code on every device, again whenever the session expires (one month at most). Too much friction. It can replace the Worker gate later with no app changes.
- **A shared password.** Rejected: one more step than a link, and it can't revoke one friend.
- **Cloudflare Pages with `functions/_middleware.js`** (shorter `minmax5e.pages.dev`). Rejected: Cloudflare steers new projects to Workers. The same files move to Pages if ever needed.
- **GitHub Pages.** Rejected: no gate, no custom headers, and private Pages needs Enterprise Cloud.
- **The gate written in Rust (`workers-rs`, crate `worker` 0.9).** Rejected: about 40 lines of standard Web Crypto JavaScript doesn't justify a second WASM build, and the gate is infrastructure, not the product.
- **A custom domain.** Declined by the owner. It can be added later without changing anything else.

Research: [11](../research/11-web-first-delivery.md)
