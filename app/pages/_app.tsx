import 'antd/dist/antd.less'
import '../styles/globals.less'
import type { AppProps } from 'next/app'
import { Provider } from 'next-auth/client'

function TiSpace({ Component, pageProps }: AppProps) {
  return (
    <Provider session={pageProps.session}>
      <Component {...pageProps} />
    </Provider>
  )
}

export default TiSpace
