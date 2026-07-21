import { getPageMap } from 'nextra/page-map'

const BASE_URL = 'https://docs.smbcloud.xyz'

/**
 * Collect every real page route from Nextra's page map.
 *
 * The map mixes three kinds of entry: `_meta` files (a bare `data` object),
 * folders (a `route` plus `children`), and pages (a `route`). Folders here use
 * `asIndexPage` frontmatter, so they resolve to a real page too and belong in
 * the sitemap — but a folder without its own page would still carry a `route`,
 * so we only emit entries Nextra gave a `name` (pages and index pages), never
 * the synthetic separators declared in `_meta.js`.
 */
function collectRoutes(pageMap, seen = new Set()) {
  for (const item of pageMap) {
    if (!item || item.data) continue
    if (typeof item.route === 'string' && item.name) {
      seen.add(item.route)
    }
    if (Array.isArray(item.children)) {
      collectRoutes(item.children, seen)
    }
  }
  return seen
}

// Shallower pages are the entry points people and crawlers land on first.
// Google largely ignores `priority`, but it costs nothing and it keeps the
// hierarchy explicit for the crawlers that still read it.
function priorityFor(route) {
  if (route === '/') return 1
  const depth = route.split('/').filter(Boolean).length
  if (depth === 1) return 0.8
  if (depth === 2) return 0.7
  return 0.6
}

export default async function sitemap() {
  const pageMap = await getPageMap()
  const routes = [...collectRoutes(pageMap)].sort()

  return routes.map(route => ({
    url: `${BASE_URL}${route === '/' ? '' : route}`,
    changeFrequency: 'weekly',
    priority: priorityFor(route)
  }))
}
