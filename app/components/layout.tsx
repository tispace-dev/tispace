import React from 'react'
import { Breadcrumb, Layout as AntDesignLayout, Menu } from 'antd'

import Header from './header'
import styles from '../styles/Layout.module.less'
import Footer from './footer'

const { Content } = AntDesignLayout

type Props = {
  children: React.ReactNode
  selectedKey: string
  breadcrumb: string
}

function Layout({ children, selectedKey, breadcrumb }: Props) {
  return (
    <AntDesignLayout className={styles.layout}>
      <Header>
        <Menu
          theme="dark"
          mode="horizontal"
          defaultSelectedKeys={[selectedKey]}
        >
          <Menu.Item key="instances">Instances</Menu.Item>
        </Menu>
      </Header>
      <Content className={styles.content}>
        <Breadcrumb className={styles.breadcrumb}>
          <Breadcrumb.Item>Home</Breadcrumb.Item>
          <Breadcrumb.Item>{breadcrumb}</Breadcrumb.Item>
        </Breadcrumb>
        {children}
      </Content>
      <Footer />
    </AntDesignLayout>
  )
}

export default Layout
