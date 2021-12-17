import type { NextPage } from 'next'
import { signIn } from 'next-auth/react'
import { Button } from 'antd'
import Head from 'next/head'

import styles from '../styles/Login.module.less'
import Footer from '../components/footer'

const Login: NextPage = () => {
  const onFinish = async () => {
    await signIn('google', {
      callbackUrl: 'window.location.origin',
    })
  }

  return (
    <div className={styles.container}>
      <Head>
        <title>TiSpace</title>
        <meta name="description" content="TiSpace" />
        <link rel="icon" href="/favicon.ico" />
      </Head>
      <main className={styles.main}>
        <h1 className={styles.title}>Welcome to TiSpace!</h1>
        <div className={styles.login}>
          <Button
            type="primary"
            htmlType="submit"
            className={styles.submit}
            onClick={onFinish}
          >
            Log in wit Google
          </Button>
        </div>
      </main>
      <Footer />
    </div>
  )
}

export default Login
