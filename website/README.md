# Mara project website (Hugo)

Static marketing and onboarding site for [Mara](https://github.com/ArdurAI/mara), following the common monorepo pattern used by Kubernetes, Prometheus, and many CNCF projects (`website/` at repo root).

## Requirements

- **Hugo** extended **0.120+** (tested with 0.161.x). Minimum is recorded in [`.hugo_build.lock`](.hugo_build.lock).
- Install: [Hugo installation](https://gohugo.io/installation/)

This directory is **standalone** from the Rust workspace: no `Cargo.toml` changes are required.

## Theme

The site uses a **custom in-repo theme** at [`themes/mara/`](themes/mara/) (deep slate background, electric cyan accent, DM Sans + JetBrains Mono, dark/light toggle). No Git submodules or external Hugo Modules are required for a clean build.

## Commands

```bash
cd website
hugo server -D
```

Open the URL Hugo prints (usually `http://localhost:1313/`).

Production build (writes to `public/`):

```bash
cd website
hugo --gc --minify
```

## Configuration

- [`hugo.toml`](hugo.toml) — set `baseURL` to your real domain before deploying (placeholder is `https://example.com/`).

## Deploy

CI can build with [peaceiris/actions-hugo](https://github.com/peaceiris/actions-hugo) (see `.github/workflows/hugo.yml`). Upload `website/public/` as a GitHub Pages artifact or sync to any static host.
