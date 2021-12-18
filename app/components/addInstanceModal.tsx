import React, { useEffect, useRef } from 'react'
import { Form, Input, InputNumber, Modal, FormInstance } from 'antd'
import { FaMemory } from 'react-icons/fa'
import { ImFloppyDisk } from 'react-icons/im'
import { BsFillCpuFill } from 'react-icons/bs'

import { InstanceRequest } from '../lib/service/instanceService'

const layout = {
  labelCol: { span: 4 },
  wrapperCol: { span: 18 },
}

// See: https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#dns-subdomain-names
const subDomainRegex = /^(?![0-9]+$)(?!.*-$)(?!-)[a-zA-Z0-9-]{1,63}$/g

interface AddInstanceModalProps {
  visible: boolean
  onCreate: (instance: InstanceRequest) => Promise<void>
  onCancel: () => void
}

const useResetFormOnCloseModal = ({
  form,
  visible,
}: {
  form: FormInstance
  visible: boolean
}) => {
  const prevVisibleRef = useRef<boolean>()
  useEffect(() => {
    prevVisibleRef.current = visible
  }, [visible])
  const prevVisible = prevVisibleRef.current

  useEffect(() => {
    if (!visible && prevVisible) {
      form.resetFields()
    }
  }, [form, prevVisible, visible])
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
          {...layout}
          form={form}
          name="add-instance"
          initialValues={{ cpu: 8, memory: 16, disk_size: 80 }}
        >
          <Form.Item
            name="name"
            label="Name"
            rules={[
              {
                required: true,
                message: 'Please input your instance name!',
              },
              {
                required: true,
                pattern: subDomainRegex,
                message:
                  'Only lowercase letters, numbers, and `-` can be included, please start and end with a lowercase letter or number!',
              },
            ]}
          >
            <Input />
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
              min={8}
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
              min={60}
              max={200}
              addonBefore={<ImFloppyDisk />}
              addonAfter="GiB"
            />
          </Form.Item>
        </Form>
      </Modal>
    </>
  )
}

export default AddInstanceModal
