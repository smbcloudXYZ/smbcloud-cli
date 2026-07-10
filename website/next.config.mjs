import nextra from 'nextra'

const withNextra = nextra({
  search: {
    codeblocks: false
  }
})

export default withNextra({
  reactStrictMode: true,
  // smbCloud Deploy (kind = "nextjs-ssr") rsyncs .next/standalone/ to the
  // server and runs server.js under PM2. Without this the standalone dir is
  // never produced and `smb deploy` aborts. See .smb/config.toml.
  output: 'standalone'
})
