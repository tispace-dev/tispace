import type { NextPage } from 'next'
import { signIn } from 'next-auth/react'
import Head from 'next/head'
import Image from 'next/image'
import { Button } from 'antd'

import styles from '../styles/Login.module.less'

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

      <footer className={styles.footer}>
        <a
          href="https://vercel.com?utm_source=create-next-app&utm_medium=default-template&utm_campaign=create-next-app"
          target="_blank"
          rel="noopener noreferrer"
        >
          Powered by{' '}
          <span className={styles.logo}>
            <Image src="/vercel.svg" alt="Vercel Logo" width={72} height={16} />
          </span>
        </a>
      </footer>
    </div>
  )
}

export default Login
