import React from 'react'
import { Form, Input, InputNumber, Modal, Select } from 'antd'
import { FaMemory } from 'react-icons/fa'
import { ImFloppyDisk } from 'react-icons/im'
import { BsFillCpuFill } from 'react-icons/bs'

import { Images, Runtimes } from './instance'
import { modalFormLayout, useResetFormOnCloseModal } from './modal'
import { CreateInstanceRequest } from '../lib/service/instanceService'

// See: https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#names
const nameRegex = /^(?![0-9]+$)(?!.*-$)(?!-)[a-z0-9-]{1,63}$/g

interface AddInstanceModalProps {
  visible: boolean
  onCreate: (instance: CreateInstanceRequest) => Promise<void>
  onCancel: () => void
}

function AddInstanceModal({
  visible,
  onCreate,
  onCancel,
}: AddInstanceModalProps) {
  const [form] = Form.useForm()

  useResetFormOnCloseModal({
    form,
    visible,
  })

  const handleOk = async () => {
    const instance = await form.validateFields()
    await onCreate(instance)
  }

  return (
    <>
      <Modal
        title="New instance"
        visible={visible}
        onOk={handleOk}
        onCancel={onCancel}
      >
        <Form
          {...modalFormLayout}
          form={form}
          name="add-instance"
          initialValues={{
            cpu: 8,
            memory: 16,
            disk_size: 80,
            image: Images.CentOS9,
            runtime: Runtimes.Kvm,
          }}
        >
          <Form.Item
            name="name"
            label="Name"
            rules={[
              {
                required: true,
                pattern: nameRegex,
                message:
                  'Only lowercase letters, numbers, and `-` can be included, please start and end with a lowercase letter or number!',
              },
            ]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="image"
            label="Image"
            rules={[
              {
                required: true,
                message: 'Please select an image!',
              },
            ]}
          >
            <Select>
              <Select.Option value={Images.CentOS9}>centos:9-Stream</Select.Option>
              <Select.Option value={Images.Ubuntu2004}>
                ubuntu:20.04
              </Select.Option>
              <Select.Option value={Images.Ubuntu2204}>
                ubuntu:22.04
              </Select.Option>
            </Select>
          </Form.Item>
          <Form.Item
            name="cpu"
            label="CPU"
            rules={[
              {
                required: true,
                message: 'Please input your instance CPU number!',
              },
            ]}
          >
            <InputNumber
              min={1}
              max={16}
              addonBefore={<BsFillCpuFill />}
              addonAfter="Core"
            />
          </Form.Item>
          <Form.Item
            name="memory"
            label="Memory"
            rules={[
              {
                required: true,
                message: 'Please input your instance memory size!',
              },
            ]}
          >
            <InputNumber
              min={1}
              max={64}
              addonBefore={<FaMemory />}
              addonAfter="GiB"
            />
          </Form.Item>
          <Form.Item
            name="disk_size"
            label="Disk Size"
            rules={[
              { required: true, message: 'Please input your disk size!' },
            ]}
          >
            <InputNumber
              min={10}
              max={500}
              addonBefore={<ImFloppyDisk />}
              addonAfter="GiB"
            />
          </Form.Item>
          <Form.Item
            name="runtime"
            label="Runtime"
            rules={[
              {
                required: true,
                message: 'Please select a runtime!',
              },
            ]}
          >
            <Select>
              <Select.Option value={Runtimes.Kvm}>kvm</Select.Option>
              <Select.Option value={Runtimes.Lxc}>lxc</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item
            name="node_name"
            label="Node Name"
            rules={[
              {
                pattern: nameRegex,
                message:
                  'Only lowercase letters, numbers, and `-` can be included, please start and end with a lowercase letter or number!',
              },
            ]}
          >
            <Input placeholder="Auto" />
          </Form.Item>
          <Form.Item
            name="storage_pool"
            label="Storage Pool"
            rules={[
              {
                pattern: nameRegex,
                message:
                  'Only lowercase letters, numbers, and `-` can be included, please start and end with a lowercase letter or number!',
              },
            ]}
          >
            <Input placeholder="Auto" />
          </Form.Item>
        </Form>
      </Modal>
    </>
  )
}

export default AddInstanceModal
