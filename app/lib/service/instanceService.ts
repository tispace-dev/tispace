import service from './index'

export type CreateInstanceRequest = {
  name: string
  cpu: number
  memory: number
  disk_size: number
  image?: string
  runtime?: string
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
}

export enum InstanceStatus {
  Starting = 'Starting',
  Running = 'Running',
  Stopping = 'Stopping',
  Stopped = 'Stopped',
  Deleting = 'Deleting',
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
