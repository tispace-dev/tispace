import React from 'react'
import { Form, InputNumber, Modal, Select } from 'antd'
import { FaMemory } from 'react-icons/fa'
import { BsFillCpuFill } from 'react-icons/bs'

import { Instance, UpdateInstanceRequest } from '../lib/service/instanceService'
import { modalFormLayout, useResetFormOnCloseModal } from './modal'
import { Runtimes } from './instance'

interface UpdateInstanceModalProps {
  visible: boolean
  instance: Instance
  onUpdate: (
    instanceName: string,
    request: UpdateInstanceRequest
  ) => Promise<void>
  onCancel: () => void
}

function UpdateInstanceModal({
  visible,
  instance,
  onUpdate,
  onCancel,
}: UpdateInstanceModalProps) {
  const [form] = Form.useForm()

  useResetFormOnCloseModal({
    form,
    visible,
  })

  const handleOk = async () => {
    const request = await form.validateFields()
    await onUpdate(instance.name, request)
  }

  return (
    <>
      <Modal
        title={`Update ${instance.name}`}
        visible={visible}
        onOk={handleOk}
        onCancel={onCancel}
      >
        <Form
          {...modalFormLayout}
          form={form}
          name="update-instance"
          initialValues={{
            cpu: instance.cpu,
            memory: instance.memory,
            runtime: instance.runtime,
          }}
        >
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
              min={8}
              max={64}
              addonBefore={<FaMemory />}
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
              <Select.Option value={Runtimes.Kata}>kata</Select.Option>
              <Select.Option value={Runtimes.Runc}>runc</Select.Option>
            </Select>
          </Form.Item>
        </Form>
      </Modal>
    </>
  )
}

export default UpdateInstanceModal
