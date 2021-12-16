import { useEffect, useState } from 'react'
import type { NextPage } from 'next'
import { useRouter } from 'next/router'
import { message, Table } from 'antd'
import { useSession } from 'next-auth/react'

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
  const shouldRedirect = !(status === 'loading' || session)

  const [instances, setInstances] = useState([])

  useEffect(() => {
    if (shouldRedirect) {
      router.push('/login')
    }

    ;(async () => {
      try {
        const instances = await listInstance()
        setInstances(instances.data.instances)
      } catch (e) {
        // TODO: Use log collection.
        console.log(e)
        message.error('List instances failed')
      }
    })()
  }, [shouldRedirect, router])

  return <Table dataSource={instances} columns={columns} />
}

export default Home
