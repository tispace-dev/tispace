import React, { useEffect, useState } from 'react'
import type { NextPage } from 'next'
import { useRouter } from 'next/router'
import { Button, Popconfirm, message, Table, Tag, Spin, Input } from 'antd'
import { useSession } from 'next-auth/react'
import { PlusOutlined, StopOutlined, CopyOutlined } from '@ant-design/icons'
import { ColumnsType } from 'antd/es/table'
import useInterval from '@use-it/interval'
import { CopyToClipboard } from 'react-copy-to-clipboard'

import Layout from '../components/layout'
import {
  createInstance,
  deleteInstance,
  Instance,
  CreateInstanceRequest,
  InstanceStatus,
  listInstances,
  startInstance,
  stopInstance,
  updateInstance,
  UpdateInstanceRequest,
  isRunnable,
} from '../lib/service/instanceService'
import AddInstanceModal from '../components/addInstanceModal'
import styles from '../styles/index.module.less'
import UpdateInstanceModal from '../components/updateInstanceModal'

const Home: NextPage = () => {
  const router = useRouter()
  const { data: session, status } = useSession()
  const [instances, setInstances] = useState([])
  const [tableVisible, setTableVisible] = useState(true)
  const [addInstanceModalVisible, setAddInstanceModalVisible] = useState(false)
  const [record, setRecord] = useState({} as Instance)
  const [updateInstanceModalVisible, setUpdateInstanceModalVisible] =
    useState(false)

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

  const handleAddInstanceOpen = () => {
    setAddInstanceModalVisible(true)
  }

  const handleAddInstanceCancel = () => {
    setAddInstanceModalVisible(false)
  }

  const handleUpdateInstanceOpen = () => {
    setUpdateInstanceModalVisible(true)
  }

  const handleUpdateInstanceCancel = () => {
    setUpdateInstanceModalVisible(false)
  }

  const handleCreate = async (instance: CreateInstanceRequest) => {
    try {
      await createInstance(instance)
      message.success('Create instance success')
      await listAllInstance()
      setAddInstanceModalVisible(false)
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

  const handleStop = (instanceName: string) => {
    ;(async () => {
      try {
        await stopInstance(instanceName)
        message.success('Stop instance success')
        await listAllInstance()
      } catch (e) {
        console.log(e)
        message.error('Stop instance failed')
      }
    })()
  }

  const handleStart = (instanceName: string) => {
    ;(async () => {
      try {
        await startInstance(instanceName)
        message.success('Start instance success')
        await listAllInstance()
      } catch (e) {
        console.log(e)
        message.error('Start instance failed')
      }
    })()
  }

  const handleUpdate = async (
    instanceName: string,
    request: UpdateInstanceRequest
  ) => {
    try {
      await updateInstance(instanceName, request)
      message.success('Update instance success')
      await listAllInstance()
      setUpdateInstanceModalVisible(false)
    } catch (e) {
      console.log(e)
      message.error('Update instance failed')
    }
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

  const getOperation = (record: Instance) => {
    const deleteInstancePopconfirm = () => {
      return (
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
      )
    }
    if (record.status === InstanceStatus.Running) {
      return (
        <div className={styles.operation}>
          {deleteInstancePopconfirm()}/
          <Popconfirm
            title="Are you sure to stop this instance?"
            onConfirm={() => {
              handleStop(record.name)
            }}
            okText="Yes"
            cancelText="No"
          >
            <a href="#">Stop</a>
          </Popconfirm>
        </div>
      )
    } else if (record.status === InstanceStatus.Starting) {
      return (
        <div className={styles.operation}>{deleteInstancePopconfirm()}</div>
      )
    } else if (record.status === InstanceStatus.Stopped) {
      return (
        <div className={styles.operation}>
          <Popconfirm
            title="Are you sure to start this instance?"
            onConfirm={() => {
              handleStart(record.name)
            }}
            okText="Yes"
            cancelText="No"
          >
            <a href="#">Start</a>
          </Popconfirm>
          /
          <a
            href="#"
            onClick={() => {
              setRecord(record)
              handleUpdateInstanceOpen()
            }}
          >
            Update
          </a>
        </div>
      )
    } else {
      return <StopOutlined />
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
      title: 'Image',
      dataIndex: 'image',
      key: 'image',
      render: (image) => {
        return (
          <a href={`https://hub.docker.com/r/${image}`}>
            <Tag color="blue">{image}</Tag>
          </a>
        )
      },
    },
    {
      title: 'Hostname',
      dataIndex: 'hostname',
      key: 'hostname',
    },
    {
      title: 'SSH Command',
      dataIndex: 'ssh_host',
      key: 'ssh_host',
      render: (_, record: Instance) => {
        if (isRunnable(record.status)) {
          if (!record.ssh_host || !record.ssh_port) {
            return <Spin />
          } else {
            const sshCommand = `ssh root@${record.ssh_host} -p ${record.ssh_port}`
            return (
              <div className={styles.ssh}>
                <span className={styles.command}>{sshCommand}</span>
                <CopyToClipboard
                  text={sshCommand}
                  onCopy={() => message.success('Command copied!')}
                >
                  <Button
                    type="dashed"
                    shape="circle"
                    icon={<CopyOutlined />}
                  />
                </CopyToClipboard>
              </div>
            )
          }
        } else {
          return '-'
        }
      },
    },
    {
      title: 'SSH Initialization Password',
      dataIndex: 'password',
      key: 'password',
      render: (password, record) => {
        if (isRunnable(record.status)) {
          if (record.status === InstanceStatus.Starting) {
            return <Spin />
          } else {
            return (
              <>
                <Input.Password
                  className={styles.password}
                  bordered={false}
                  value={password}
                  visibilityToggle={false}
                />
                <CopyToClipboard
                  text={password}
                  onCopy={() => message.success('Password copied!')}
                >
                  <Button
                    type="dashed"
                    shape="circle"
                    icon={<CopyOutlined />}
                  />
                </CopyToClipboard>
              </>
            )
          }
        } else {
          return '-'
        }
      },
    },
    {
      title: 'Runtime',
      dataIndex: 'runtime',
      key: 'runtime',
    },
    {
      title: 'Status',
      dataIndex: 'status',
      key: 'status',
      render: (status) => getStatusTag(status),
    },
    {
      title: 'Operation',
      dataIndex: 'name',
      key: 'name',
      render: (_, record: Instance) => {
        return getOperation(record)
      },
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
        setTableVisible(false)
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
        onClick={handleAddInstanceOpen}
      >
        New instance
      </Button>
      <AddInstanceModal
        visible={addInstanceModalVisible}
        onCreate={handleCreate}
        onCancel={handleAddInstanceCancel}
      />
      <Table
        loading={tableVisible}
        dataSource={instances}
        columns={columns}
        rowKey="name"
        bordered
        scroll={{ x: 1300 }}
      />
      <UpdateInstanceModal
        visible={updateInstanceModalVisible}
        onUpdate={handleUpdate}
        instance={record}
        onCancel={handleUpdateInstanceCancel}
      />
    </Layout>
  )
}

export default Home
