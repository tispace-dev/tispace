export enum Images {
  Centos7 = 'tispace/centos7',
  Ubuntu2004 = 'tispace/ubuntu2004',
}

export enum Runtimes {
  Kata = 'kata',
  Runc = 'runc',
}

// See: https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#names
export const instanceNameRegex = /^(?![0-9]+$)(?!.*-$)(?!-)[a-z0-9-]{1,63}$/g
