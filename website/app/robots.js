const BASE_URL = 'https://docs.smbcloud.xyz'

// A blanket allow already covers the AI answer-engine crawlers (GPTBot,
// ClaudeBot, PerplexityBot, and friends) as well as classic search bots, so
// they can cite these docs. Naming individual agents is only worth doing to
// treat one differently — that's a content-licensing call, not a default.
export default function robots() {
  return {
    rules: [{ userAgent: '*', allow: '/' }],
    sitemap: `${BASE_URL}/sitemap.xml`,
    host: BASE_URL
  }
}
