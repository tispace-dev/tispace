# TiSpace Introduction

## Summary

In this project, we will build a system that supports developers to use the K8s Pod as a virtual machine.

The system has the following features:

- Flexible and user-friendly resource management platform
- Highly available and schedulable disk services
- Secure Container Runtime for Lightweight Virtual Machines
- Delivers consistent performance as standard Linux containers


## Motivation

At PingCAP, our developers use a lot of virtual machines for our development and testing work. However, these virtual machines have poor performance and they are heavily overdraft. In addition, at PingCAP we do not have a good management platform to distribute and manage these VMs for developers. Developers manage and request these machine resources by asking each other. 

To optimize these inefficient processes, we decided to develop a management platform to efficiently manage and allocate machine resources.

## Detailed design

The whole system uses [K8s] as the infrastructure, and deploys [ceph] clusters in [K8s] for disk high availability, and replaces the [K8s] container layer with [kata] to provide good virtualization capability.

<!-- TODO: Add architecture diagram -->

### K8s

We have deployed [K8s] clusters on bare machine and will rely on [K8s] to provide the scheduling capabilities for the entire system. With this, users only need to provide resource requests when creating instances, and we can use [K8s] scheduling capabilities to create the corresponding Pod resources for users.

### ceph

[Ceph] is a distributed storage system that supports object, block, and file storage interfaces. We built a [ceph] cluster using [Rook] in the [K8s] cluster, using it as a persistent volume resource, and assigning it to each Pod through a persistent volume claim when a user requests an instance. This way, the user's data is automatically guaranteed to be highly available, and the disk resources have the ability to be scheduled.

### kata

In order to give users a full VM experience, we also needed to provide users with the ability to run docker inside the instance. if we used docker directly at the container layer, we would need to enable privileged mode to allow users to use docker properly, but enabling privileged mode would lead to additional security and resource control risks. So we chose to replace the container layer with [kata], which provides good isolation while maintaining the same performance as a standard Linux container.

### Frontend

In order to provide a good interactive experience for users, we built web pages where users can quickly create and destroy instances by logging in with their Google accounts.

### Backend

On the backend, we implement an operator-like service application that receives front-end HTTP requests and creates the corresponding Pod based on the requested resource information.

## Future possibilities

- Expose K8s capabilities to users so that they can use the tidb-operator directly.
- Better integration with TiUP and the ability to quickly deploy clusters with TiUP.
- Custom images are supported, and developers can customize their own workflow and toolsets.


[k8s]: https://github.com/kubernetes/kubernetes/
[ceph]: https://github.com/ceph/ceph
[kata]: https://github.com/kata-containers/kata-containers
[rook]: https://github.com/rook/rook