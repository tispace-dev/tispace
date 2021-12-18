import service from './index'

export type InstanceRequest = {
  name: string
  cpu: number
  memory: number
  disk_size: number
}

export async function listInstances() {
  return await service.get('/instances')
}

export async function createInstance(instance: InstanceRequest) {
  return await service.post('/instances', instance)
}

export async function deleteInstance(instanceName: string) {
  return await service.delete(`/instances/${instanceName}`)
}
