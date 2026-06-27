#!/usr/bin/env node
/**
 * Sync the /developer docs from the smbcloud-cli repository.
 *
 * The CLI / tooling documentation lives in `docs/` of smbcloud-cli and is the
 * source of truth. This script pulls those files into content/developer/ so the
 * docs site tracks the tool. Synced files are committed (vendored) so the site
 * builds on CI without network access — re-run this script to refresh them.
 *
 *   pnpm sync:developer
 *
 * Hand-written index pages (content/developer/**\/index.mdx) are NOT touched.
 * Only the files listed in MANIFEST and the generated _meta.js are written.
 */
import { execSync } from 'node:child_process'
import { mkdtempSync, rmSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO = 'https://github.com/smbcloudXYZ/smbcloud-cli.git'
const BRANCH = 'main'
const SRC_DIR = 'docs'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const DEST = join(ROOT, 'content', 'developer')

// Source file -> where it lands and how it's labelled in the sidebar.
// `mail-service.md` is intentionally excluded: it's a scratch notes file in the
// source repo, not publishable documentation.
const MANIFEST = {
  cli: {
    title: 'CLI',
    pages: [
      { src: 'cli-install.md', slug: 'native', title: 'Native (Cargo)' },
      { src: 'cli-install-homebrew.md', slug: 'homebrew', title: 'Homebrew' },
      { src: 'cli-install-npm.md', slug: 'npm', title: 'npm' },
      { src: 'cli-install-pypi.md', slug: 'pypi', title: 'PyPI' }
    ]
  },
  contributing: {
    title: 'Contributing',
    pages: [
      { src: 'debugging.md', slug: 'debugging', title: 'VSCode Debugging' },
      { src: 'development.md', slug: 'development', title: 'Releasing & ADRs' },
      { src: 'gems.md', slug: 'gems', title: 'Publishing the gem' }
    ]
  }
}

const blobUrl = src =>
  `https://github.com/smbcloudXYZ/smbcloud-cli/blob/${BRANCH}/${SRC_DIR}/${src}`

/** Wrap synced markdown: header note + a source callout after the first H1. */
function decorate(markdown, src) {
  const note = `{/* AUTO-GENERATED — do not edit. Synced by scripts/sync-developer-docs.mjs from ${SRC_DIR}/${src} (smbcloud-cli, ${BRANCH}). */}`
  const callout = [
    "import { Callout } from 'nextra/components'",
    '',
    '<Callout type="info">',
    `  Synced from [\`${SRC_DIR}/${src}\`](${blobUrl(src)}) on the \`${BRANCH}\` branch of`,
    '  [smbcloud-cli](https://github.com/smbcloudXYZ/smbcloud-cli). Edit it there.',
    '</Callout>'
  ].join('\n')

  const lines = markdown.replace(/\r\n/g, '\n').trimEnd().split('\n')
  const h1 = lines.findIndex(l => /^#\s/.test(l))
  if (h1 === -1) {
    return `${note}\n\n${callout}\n\n${lines.join('\n')}\n`
  }
  const before = lines.slice(0, h1 + 1).join('\n')
  const after = lines.slice(h1 + 1).join('\n')
  return `${note}\n\n${before}\n\n${callout}\n${after}\n`
}

// Note: the folder's own index page uses `asIndexPage` frontmatter, so it must
// NOT be listed as an `index` key here — Nextra surfaces it via the parent.
function metaFile(pages) {
  const entries = pages.map(p => `  ${p.slug}: ${JSON.stringify(p.title)},`)
  return `export default {\n${entries.join('\n')}\n}\n`
}

function main() {
  const tmp = mkdtempSync(join(tmpdir(), 'smb-cli-docs-'))
  try {
    console.log(`Cloning ${REPO} (${BRANCH}, sparse: ${SRC_DIR})…`)
    execSync(
      `git clone --depth 1 --branch ${BRANCH} --filter=blob:none --sparse ${REPO} "${tmp}"`,
      { stdio: 'inherit' }
    )
    execSync(`git -C "${tmp}" sparse-checkout set ${SRC_DIR}`, { stdio: 'inherit' })

    let count = 0
    for (const [section, { title, pages }] of Object.entries(MANIFEST)) {
      const dir = join(DEST, section)
      mkdirSync(dir, { recursive: true })
      for (const page of pages) {
        const raw = readFileSync(join(tmp, SRC_DIR, page.src), 'utf8')
        writeFileSync(join(dir, `${page.slug}.mdx`), decorate(raw, page.src))
        count++
      }
      writeFileSync(join(dir, '_meta.js'), metaFile(pages))
      console.log(`  ${section}: ${pages.length} pages (${title})`)
    }

    // Top-level developer nav: each section (the overview is asIndexPage).
    const top = []
    for (const [section, { title }] of Object.entries(MANIFEST)) {
      top.push(`  ${section}: ${JSON.stringify(title)},`)
    }
    writeFileSync(join(DEST, '_meta.js'), `export default {\n${top.join('\n')}\n}\n`)

    console.log(`\nSynced ${count} pages into content/developer/.`)
  } finally {
    rmSync(tmp, { recursive: true, force: true })
  }
}

main()
