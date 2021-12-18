import React, { useEffect, useState } from 'react'
import type { NextPage } from 'next'
import { useRouter } from 'next/router'
import { message, Table } from 'antd'
import { useSession } from 'next-auth/react'

import Layout from '../components/layout'
import { listInstance } from '../lib/service/instanceService'

const columns = [
  {
    title: 'Name',
    dataIndex: 'name',
    key: 'name',
  },
  {
    title: 'CPU',
    dataIndex: 'cpu',
    key: 'cpu',
  },
  {
    title: 'Memory',
    dataIndex: 'memory',
    key: 'memory',
  },
  {
    title: 'Disk Size',
    dataIndex: 'disk_size',
    key: 'disk_size',
  },
  {
    title: 'Hostname',
    dataIndex: 'hostname',
    key: 'hostname',
  },
  {
    title: 'Status',
    dataIndex: 'status',
    key: 'status',
  },
]

const Home: NextPage = () => {
  const router = useRouter()
  const { data: session, status } = useSession()

  const [instances, setInstances] = useState([])

  useEffect(() => {
    ;(async () => {
      const shouldRedirect =
        !(status === 'loading' || session) ||
        (session && session.error === 'RefreshIdTokenError')

      if (shouldRedirect) {
        await router.push('/login')
      }

      try {
        const instances = await listInstance()
        setInstances(instances.data.instances)
      } catch (e) {
        // TODO: Use log collection.
        console.log(e)
        message.error('List instances failed')
      }
    })()
  }, [session, status, router])

  return (
    <Layout selectedKey={'instances'} breadcrumb={'Instances'}>
      <Table dataSource={instances} columns={columns} />
    </Layout>
  )
}

export default Home
