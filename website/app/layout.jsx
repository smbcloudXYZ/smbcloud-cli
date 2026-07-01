import { Footer, Layout, Navbar } from 'nextra-theme-docs'
import { Head } from 'nextra/components'
import { getPageMap } from 'nextra/page-map'
import 'nextra-theme-docs/style.css'

export const metadata = {
  metadataBase: new URL('https://docs.smbcloud.xyz'),
  title: {
    default: 'smbCloud Documentation',
    template: '%s – smbCloud Documentation'
  },
  description: 'smbCloud Documentation',
  applicationName: 'smbCloud Documentation',
  openGraph: {
    title: 'smbCloud Documentation',
    description: 'smbCloud Documentation',
    url: 'https://docs.smbcloud.xyz',
    siteName: 'smbCloud Documentation'
  }
}

const navbar = (
  <Navbar
    logo={<span>smbCloud Documentation</span>}
    projectLink="https://github.com/smbcloudXYZ"
  />
)

const footer = (
  <Footer>
    MIT {new Date().getFullYear()} ©{' '}
    <a href="https://docs.smbcloud.xyz" target="_blank" rel="noreferrer">
      smbCloud Documentation
    </a>
  </Footer>
)

export default async function RootLayout({ children }) {
  const pageMap = await getPageMap()
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head />
      <body>
        <Layout
          navbar={navbar}
          footer={footer}
          pageMap={pageMap}
          docsRepositoryBase="https://github.com/smbcloudXYZ/smbcloud-docs/tree/main"
          editLink="Edit this page on GitHub"
          sidebar={{ defaultMenuCollapseLevel: 1 }}
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
