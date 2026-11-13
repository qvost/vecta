# Vecta Web Viewer

Standalone Vite application. All report processing happens in the browser. No API,
analytics, CDN fonts, or database is required. Fonts are bundled from npm packages.

```sh
npm install
npm test
npm run build
npm run dev
```

Production output is `dist/`; serve it with any static host over HTTPS. The viewer
opens core-generated `#z=` links (URL-safe Base64, raw DEFLATE, schema v1) and CLI
`vecta diff --format json` or `vecta gate --format json` reports. Open report accepts
a share URL, pasted JSON, or a JSON file. Downloads preserve loaded report metadata.
The default scene is explicitly labeled demonstration data. A shared summary has
no gate verdict unless the payload includes one; increased size alone is never
presented as a policy failure. Omitted-row counts and incomplete ABI data remain
visible. The dial illustrates the recorded transition; it does not infer OS support.

Browser requirements: modern ES modules and `DecompressionStream('deflate-raw')`.
Unsupported browsers can still import JSON. Inflated payloads are capped at 2 MiB;
unsafe integer values and unsupported schemas are rejected rather than rounded.
There are no runtime requests to fetch report contents from pasted URLs.

Motion uses GSAP / ScrollTrigger and Lenis, with cleanup when reports change and
live reduced-motion support. Cards pin on scroll, heading words flip in 3D, and
the SVG runtime needle scrubs through the compatibility scene. The amber grid
preloader has a CSS timeout and never blocks interaction. Semantic tables, native
dialog focus handling, keyboard controls and narrow-screen layouts are included.

Implementation follows the supplied House of Epochs aesthetic description; the
reference package was not fetched in the offline environment. No dependencies
were installed and no production build or browser visual check was performed
locally. The host runner should install dependencies, run tests and build before
publishing. Local validation: Node syntax checks and payload tests.
