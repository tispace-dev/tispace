import axios from 'axios'
import { notification } from 'antd'
import { getSession } from 'next-auth/react'

const service = axios.create({
  baseURL: process.env.NEXT_PUBLIC_SERVER_URL,
})

service.interceptors.request.use(async (config) => {
  const session = await getSession()

  if (config.headers) {
    config.headers.Authorization = `Bearer ${session?.id_token}`
  }

  return config
})

service.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.code === 'ECONNABORTED' || !error.response) {
      notification.warning({
        key: 'network-error',
        message: 'Sorry',
        description: 'Please check if your network is working...',
      })
    }

    if (error.response) {
      switch (error.response.status) {
        case 401:
          notification.warning({
            key: 'status-401',
            message:
              error.response.data.error ||
              'The system does not seem to know you',
            description: 'You need to log in',
          })
          break
        case 403:
          notification.warning({
            key: 'status-403',
            message: 'Sorry',
            description: 'You do not have permission',
          })
          break
        case 400:
          notification.warning({
            key: 'status-400',
            message: 'Sorry',
            description: error.response.data.error,
          })
          break
        case 404:
          notification.warning({
            key: 'status-404',
            message: 'Sorry',
            description: 'You visit the barren land',
          })
          break
        case 500:
          notification.warning({
            message: 'Sorry',
            description: 'There seems to be a problem with the server',
          })
          break
        default:
          notification.open({
            message: 'Attention',
            description: error.response.data.error,
          })
          break
      }

      return Promise.reject(error)
    }
  }
)

export default service
