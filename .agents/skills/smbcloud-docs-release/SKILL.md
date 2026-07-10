---
name: smbcloud-docs-release
description: Use when building, generating, or releasing the smbCloud documentation site (`website/`, Nextra + Next.js) — the developer-docs generation pipeline that reads the sibling `docs/` tree, the `nextjs-ssr` deploy via `smb deploy`, and the public-repo content boundary.
---

# smbCloud Docs Release

Use this skill when work touches the documentation site that lives at `website/`
in this repo (Nextra 4 on Next.js 16, indexed with Pagefind).

Applies to:

- the developer-docs generation pipeline (`website/scripts/generate-developer-docs.mjs`)
- editing or adding pages under `website/content/`
- the source docs under `docs/` (the single source of truth for `/developer`)
- deploying the site with `smb deploy` (`kind = "nextjs-ssr"`)
- the public-repo content boundary for anything published to the docs site

For the mechanics of the `nextjs-ssr` deploy path itself (standalone build,
rsync layout, PM2, nginx), read the `smbcloud-deploy-nextjs` skill — this skill
only covers what is docs-specific.

## Layout

```
smbcloud-cli/
├── docs/        # plain Markdown — the source of truth for /developer pages
└── website/     # the Nextra site (Next.js 16, src-less app/ layout)
    ├── content/                 # MDX pages: about, auth, deploy, gresiq, mail, developer/
    │   └── developer/           # index.mdx hand-written; cli/ + contributing/ generated
    ├── scripts/generate-developer-docs.mjs
    └── next.config.mjs          # output: 'standalone' (required by nextjs-ssr deploy)
```

`docs/` sits next to `website/` at the repo root. The site reads it **in place**
— no git clone, no network fetch.

## Developer-docs generation

`/developer` CLI and contributing pages are **generated**, not hand-edited.
`website/scripts/generate-developer-docs.mjs` reads files from `../docs`, wraps
each with an "auto-generated, edit at source" callout linking back to the
`docs/` file on GitHub, and writes `.mdx` plus `_meta.js` into
`content/developer/{cli,contributing}/`.

Key rules:

- **`docs/` is the single source of truth.** Edit the Markdown in `docs/`, never
  the generated `.mdx` under `content/developer/cli|contributing/`.
- **Generated files are gitignored.** See `website/.gitignore` — the generated
  `.mdx` and `_meta.js` are excluded so they never drift in git. Do not commit them.
- **Hand-written index pages are never touched.** `content/developer/index.mdx`
  (and any section `index.mdx`) use `asIndexPage` frontmatter and are not in the
  manifest. The script only writes the files listed in `MANIFEST` and the
  generated `_meta.js`.
- **Adding a page:** add the `docs/*.md` source, then add an entry to `MANIFEST`
  in the generator (`src`, `slug`, `title`) under `cli` or `contributing`. Then
  add the new generated path to `website/.gitignore`.
- **Excluded sources:** not every file in `docs/` is published — e.g. scratch
  notes are intentionally left out of `MANIFEST`. Only manifest entries ship.

The generator runs automatically:

```jsonc
"predev":   "pnpm docs:generate",   // before `next` dev
"prebuild": "pnpm docs:generate",   // before `next build`
"docs:generate": "node scripts/generate-developer-docs.mjs"
```

You rarely run it by hand; `pnpm dev` and `pnpm build` regenerate first. Run
`pnpm docs:generate` manually only to inspect the output.

## Build and local preview

From `website/`:

```sh
pnpm install
pnpm dev      # regenerates developer docs, then `next` dev
pnpm build    # regenerates, `next build`, then Pagefind indexes the output
```

`postbuild` runs Pagefind over `.next/server/app` and writes the search index to
`public/_pagefind`. A green `pnpm build` should report the expected page count
from Pagefind — a drop usually means a generated page failed to render or a
manifest entry was dropped.

`next.config.mjs` sets `output: 'standalone'`. Do not remove it — the
`nextjs-ssr` deploy path rsyncs `.next/standalone/` and aborts without it.

## Releasing (deploy)

The site ships through the CLI's `nextjs-ssr` deploy path, **not** through the
crate/npm/PyPI release flow (that is `smbcloud-cli-release`, a different thing).

```sh
smb deploy --project <docs-project-name>
```

Because this site is `kind = "nextjs-ssr"`, `smb deploy`:

1. runs the build locally (which regenerates the developer docs via `prebuild`),
2. rsyncs `.next/standalone/` + `.next/static/` + `public/` to the server,
3. restarts the app under PM2 over SSH.

The operational deploy fields (port, remote path, PM2 name, workspace/app IDs)
are **server-side config**, not in the committed `.smb/config.toml` — see the
public-repo boundary below. `smb deploy` fetches them from the API and merges
over the minimal local config. See `smbcloud-deploy-nextjs` for the full deploy
mechanics and `smbcloud-deploy-monorepo` if deploying as a named sub-project.

## Public-repo content boundary

**This repo is public**, so everything in `docs/` and `website/content/` is
world-readable and permanent. Read the root `CLAUDE.md` boundary section before
publishing. In docs content specifically, keep out:

- real customer/app domains, server hostnames/IPs, internal ports, PM2 names
- account/user IDs, workspace/project IDs, `frontend_app_id`/`deploy_repo_id`
- account-scoped SSH key names (`id_<n>@smbcloud`) and the production port registry
- secrets, tokens, `.env` values, real auth/CORS origins
- named incidents or examples tied to a real tenant/customer

Write generically with placeholders (`example.com`, `<app>`, `<port>`, `<n>`).
The base API host (`api.smbcloud.xyz`) is already in the CLI source, so it is not
a new leak; the items above are. Operational deploy config stays off-disk
(server-side); the committed `.smb/config.toml` is gitignored for the same reason.

## Common mistakes

- editing a generated `content/developer/{cli,contributing}/*.mdx` instead of the
  `docs/*.md` source — your change is overwritten on the next build
- committing generated developer docs (they are gitignored on purpose)
- adding a `docs/` page but forgetting the `MANIFEST` entry (it never ships) or
  the `website/.gitignore` line (it gets committed)
- removing `output: 'standalone'` from `next.config.mjs` — breaks the deploy
- writing real domains, ports, IDs, or secrets into published docs content
- confusing docs release (`smb deploy` nextjs-ssr) with the CLI crate/npm/PyPI
  release flow (`smbcloud-cli-release`)
