import React, { useEffect, useState } from 'react'
import type { NextPage } from 'next'
import { useRouter } from 'next/router'
import { Button, Popconfirm, message, Table, Tag } from 'antd'
import { useSession } from 'next-auth/react'
import { PlusOutlined, StopOutlined } from '@ant-design/icons'
import { ColumnsType } from 'antd/es/table'
import useInterval from '@use-it/interval'

import Layout from '../components/layout'
import {
  createInstance,
  deleteInstance,
  InstanceRequest,
  listInstances,
} from '../lib/service/instanceService'
import AddInstanceModal from '../components/addInstanceModal'
import styles from '../styles/index.module.less'

type Instance = {
  name: string
  cpu: number
  memory: number
  disk_size: number
  status: string
}

enum InstanceStatus {
  Starting = 'Starting',
  Running = 'Running',
  Stopping = 'Stopping',
  Stopped = 'Stopped',
  Deleting = 'Deleting',
}

const Home: NextPage = () => {
  const router = useRouter()
  const { data: session, status } = useSession()
  const [instances, setInstances] = useState([])
  const [visible, setVisible] = useState(false)

  const listAllInstance = async () => {
    try {
      const instances = await listInstances()
      setInstances(instances.data.instances)
    } catch (e) {
      // TODO: Use log collection.
      console.log(e)
      message.error('List instances failed')
    }
  }

  const handleOpen = () => {
    setVisible(true)
  }

  const handleCancel = () => {
    setVisible(false)
  }

  const handleCreate = async (instance: InstanceRequest) => {
    try {
      await createInstance(instance)
      message.success('Create instance success')
      await listAllInstance()
      setVisible(false)
    } catch (e) {
      console.log(e)
      message.error('Create instance failed')
    }
  }

  const handleDelete = (instanceName: string) => {
    ;(async () => {
      try {
        await deleteInstance(instanceName)
        message.success('Delete instance success')
        await listAllInstance()
      } catch (e) {
        console.log(e)
        message.error('Delete instance failed')
      }
    })()
  }

  const getStatusTag = (status: string) => {
    switch (status) {
      case InstanceStatus.Starting: {
        return <Tag color="lime">{status}</Tag>
      }
      case InstanceStatus.Running: {
        return <Tag color="green">{status}</Tag>
      }
      case InstanceStatus.Stopping: {
        return <Tag color="orange">{status}</Tag>
      }
      case InstanceStatus.Stopped: {
        return <Tag color="gold">{status}</Tag>
      }
      case InstanceStatus.Deleting: {
        return <Tag color="red">{status}</Tag>
      }
    }
  }

  const columns: ColumnsType<Instance> = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: 'CPU',
      dataIndex: 'cpu',
      key: 'cpu',
      sorter: (a, b) => a.cpu - b.cpu,
    },
    {
      title: 'Memory(GiB)',
      dataIndex: 'memory',
      key: 'memory',
      sorter: (a, b) => a.memory - b.memory,
    },
    {
      title: 'Disk Size(GiB)',
      dataIndex: 'disk_size',
      key: 'disk_size',
      sorter: (a, b) => a.disk_size - b.disk_size,
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
      render: (status) => getStatusTag(status),
    },
    {
      title: 'Operation',
      dataIndex: 'operation',
      render: (_, record: Instance) =>
        record.status === InstanceStatus.Running ? (
          <Popconfirm
            title="Are you sure to delete this instance?"
            onConfirm={() => {
              handleDelete(record.name)
            }}
            okText="Yes"
            cancelText="No"
          >
            <a href="#">Delete</a>
          </Popconfirm>
        ) : (
          <StopOutlined />
        ),
    },
  ]

  useEffect(() => {
    ;(async () => {
      if (status === 'loading') {
        return
      }
      const shouldRedirect =
        !session || (session && session.error === 'RefreshIdTokenError')
      if (shouldRedirect) {
        await router.push('/login')
      } else {
        await listAllInstance()
      }
    })()
  }, [session, status, router])

  useInterval(async () => {
    await listAllInstance()
  }, 30000)

  return (
    <Layout selectedKey={'instances'} breadcrumb={'Instances'}>
      <Button
        className={styles.add}
        type="primary"
        shape="round"
        size={'large'}
        icon={<PlusOutlined />}
        onClick={handleOpen}
      >
        New instance
      </Button>
      <AddInstanceModal
        visible={visible}
        onCreate={handleCreate}
        onCancel={handleCancel}
      />
      <Table dataSource={instances} columns={columns} rowKey="name" bordered />
    </Layout>
  )
}

export default Home
