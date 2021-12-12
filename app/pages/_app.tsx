import 'antd/dist/antd.less'
import '../styles/globals.less'
import type { AppProps } from 'next/app'

function TiSpace({ Component, pageProps }: AppProps) {
  return <Component {...pageProps} />
}

export default TiSpace
