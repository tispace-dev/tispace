import service from './index'

export type CreateInstanceRequest = {
  name: string
  cpu: number
  memory: number
  disk_size: number
  image?: string
  runtime?: string
  node_name?: string
  storage_pool?: string
}

export type UpdateInstanceRequest = {
  cpu: number
  memory: number
  runtime: string
}

export type Instance = {
  name: string
  cpu: number
  memory: number
  disk_size: number
  status: string
  // Deprecated: use external_ip instead.
  ssh_host: string
  // Deprecated: use 22 instead.
  ssh_port: number
  password: string
  image: string
  external_ip: string
  runtime: string
  node_name: string
  storage_pool: string
}

export enum InstanceStatus {
  Creating = 'Creating',
  Starting = 'Starting',
  Running = 'Running',
  Stopping = 'Stopping',
  Stopped = 'Stopped',
  Deleting = 'Deleting',
}

export const isRunnable = (status: string) => {
  return (
    status !== InstanceStatus.Stopping &&
    status !== InstanceStatus.Stopped &&
    status !== InstanceStatus.Deleting
  )
}

export async function listInstances() {
  return await service.get('/instances')
}

export async function createInstance(instance: CreateInstanceRequest) {
  return await service.post('/instances', instance)
}

export async function deleteInstance(instanceName: string) {
  return await service.delete(`/instances/${instanceName}`)
}

export async function stopInstance(instanceName: string) {
  return await service.post(`/instances/${instanceName}/stop`)
}

export async function startInstance(instanceName: string) {
  return await service.post(`/instances/${instanceName}/start`)
}

export async function updateInstance(
  instanceName: string,
  request: UpdateInstanceRequest
) {
  return await service.patch(`/instances/${instanceName}`, request)
}
