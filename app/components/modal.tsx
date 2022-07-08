import { FormInstance } from 'antd'
import { useEffect, useRef } from 'react'

export const modalFormLayout = {
  labelCol: { span: 5 },
  wrapperCol: { span: 18 },
}

export const useResetFormOnCloseModal = ({
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
