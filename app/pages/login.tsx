import type { NextPage } from 'next'
import { signIn } from 'next-auth/client'
import { useRouter } from 'next/router'
import Head from 'next/head'
import Image from 'next/image'
import { Form, Input, Button, message } from 'antd'
import { UserOutlined, LockOutlined } from '@ant-design/icons'

import styles from '../styles/Login.module.less'

interface LoginCredentials {
  username: string
  password: string
}

const Login: NextPage = () => {
  const router = useRouter()

  const onFinish = async (credentials: LoginCredentials) => {
    const res = await signIn('credentials', {
      username: credentials.username,
      password: credentials.password,
      callbackUrl: window.location.origin,
      redirect: false,
    })
    if (res?.error) {
      message.error(res.error)
    }

    if (res?.ok && res?.url) {
      await router.push(res.url)
    }
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
          <Form
            name="login"
            initialValues={{ remember: true }}
            onFinish={onFinish}
          >
            <Form.Item
              name="username"
              rules={[
                { required: true, message: 'Please input your Username!' },
              ]}
            >
              <Input prefix={<UserOutlined />} placeholder="Username" />
            </Form.Item>
            <Form.Item
              name="password"
              rules={[
                { required: true, message: 'Please input your Password!' },
              ]}
            >
              <Input
                prefix={<LockOutlined />}
                type="password"
                placeholder="Password"
              />
            </Form.Item>
            <Form.Item>
              <Button
                type="primary"
                htmlType="submit"
                className={styles.submit}
              >
                Log in
              </Button>
            </Form.Item>
          </Form>
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
