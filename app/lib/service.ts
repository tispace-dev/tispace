import axios from 'axios'
import { getSession } from 'next-auth/react'

const service = axios.create({
  baseURL: process.env.SERVER_URL,
})

service.interceptors.request.use(async (config) => {
  const session = await getSession()

  if (config.headers) {
    config.headers.Authorization = `Bearer ${session?.id_token}`
  }

  return config
})
