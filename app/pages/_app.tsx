import 'antd/dist/antd.less'
import '../styles/globals.less'
import type { AppProps } from 'next/app'
import { SessionProvider } from 'next-auth/react'

function TiSpace({ Component, pageProps }: AppProps) {
  return (
    <SessionProvider session={pageProps.session}>
      <Component {...pageProps} />
    </SessionProvider>
  )
}

export default TiSpace
