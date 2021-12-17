import service from './index'

export async function listInstance() {
  return await service.get('/instances')
}
