import { useEffect } from 'react'
import type { NextPage } from 'next'
import { useRouter } from 'next/router'
import { Table } from 'antd'
import { useSession } from 'next-auth/react'

const columns = [
  {
    title: 'Name',
    dataIndex: 'name',
    key: 'name',
  },
  {
    title: 'Age',
    dataIndex: 'age',
    key: 'age',
  },
  {
    title: 'Address',
    dataIndex: 'address',
    key: 'address',
  },
]

const Home: NextPage = () => {
  const router = useRouter()
  const { data: session, status } = useSession()
  const shouldRedirect = !(status === 'loading' || session)

  useEffect(() => {
    if (shouldRedirect) {
      router.push('/login')
    }
  }, [shouldRedirect, router])

  return <Table columns={columns} />
}

export default Home
