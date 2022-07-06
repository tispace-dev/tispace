export enum Images {
  CentOS7 = 'centos:7',
  Ubuntu2004 = 'ubuntu:20.04',
  Ubuntu2204 = 'ubuntu:22.04'
}

export enum Runtimes {
  Kata = 'kata',
  Runc = 'runc',
  Lxc = 'lxc',
  Kvm = 'kvm',
}

// See: https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#names
export const instanceNameRegex = /^(?![0-9]+$)(?!.*-$)(?!-)[a-z0-9-]{1,63}$/g
