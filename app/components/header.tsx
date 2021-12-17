import { Layout } from 'antd'

import React from 'react'
import styles from '../styles/Header.module.less'

const { Header: AntDesignHeader } = Layout

type Props = {
  children: React.ReactNode
}

function Header({ children }: Props) {
  return (
    <AntDesignHeader>
      <div className={styles.logo}>
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img src="/logo.svg" alt="Logo" />
      </div>
      <title>TiSpace</title>
      <meta name="description" content="TiSpace" />
      <link rel="icon" href="/favicon.ico" />
      {children}
    </AntDesignHeader>
  )
}

export default Header
