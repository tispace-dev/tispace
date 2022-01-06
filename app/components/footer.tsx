import { Layout } from 'antd'
import { RocketOutlined } from '@ant-design/icons'

import styles from '../styles/footer.module.less'
import React from 'react'

const { Footer: AntDesignFooter } = Layout

function Footer() {
  return (
    <AntDesignFooter className={styles.footer}>
      <RocketOutlined /> TiSpace ©2022
    </AntDesignFooter>
  )
}

export default Footer
