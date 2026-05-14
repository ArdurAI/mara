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

- [`hugo.toml`](hugo.toml) — `baseURL` defaults to the GitHub Pages project URL (`https://ardurai.github.io/mara/`). Update it if you move hosting or attach a custom domain.

## Deploy

CI builds with [peaceiris/actions-hugo](https://github.com/peaceiris/actions-hugo) and, on pushes to **`main`** (not PRs), publishes to **GitHub Pages** via `.github/workflows/hugo.yml`.

**One-time repo setup:** Settings → Pages → Build and deployment → Source: **GitHub Actions**.

After a successful deploy from `main`, the site is at **https://ardurai.github.io/mara/**.

You can still run `hugo --gc --minify` locally and sync `website/public/` elsewhere if needed.
