import { Layout } from 'antd'
import React from 'react'
import { RocketOutlined, LogoutOutlined } from '@ant-design/icons'
import { signOut } from 'next-auth/react'

import styles from '../styles/header.module.less'

const { Header: AntDesignHeader } = Layout

type Props = {
  children: React.ReactNode
}

const logOut = async () => {
  await signOut({
    callbackUrl: `${window.location.origin}/login`,
  })
}

function Header({ children }: Props) {
  return (
    <AntDesignHeader className={styles.header}>
      <div className={styles.logo}>
        <RocketOutlined />
        TiSpace
      </div>
      <div className={styles.logout}>
        <LogoutOutlined onClick={logOut} />
      </div>
      <title>TiSpace</title>
      <meta name="description" content="TiSpace" />
      <link rel="icon" href="/favicon.ico" />
      {children}
    </AntDesignHeader>
  )
}

export default Header
