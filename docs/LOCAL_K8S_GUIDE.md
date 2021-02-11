# Run whackadep locally with kubernetes!

## Installation Steps and Pre-Reqs

### Install docker

[Docker link](https://docs.docker.com/get-docker/)

### Install KIND*

[KIND Website](https://kind.sigs.k8s.io/)

* You do not need to use KIND, but you will have to figure out how to setup a local registry with your local kubernetes instance. (The makefile target 'launch-local-kind-cluster' handles this for KIND clusters)

## Running Things

### Setting up the cluster

Run `make launch-local-kind-cluster`

### Building the docker images

Run `make create-docker-images`

### Push Docker Images to local registry

Run `make push-docker-images`

### Apply kubernetes manifests to local cluster

Run `make apply-k8s`


## Checking on Things

### Port Forwarding

Forward various ports for local interaction with the kubernetes cluster.

Run the following `make` targets in separate terminal shells
- `make port-forward-backend` [localhost:8081](https://localhost:8081)
- `make port-forward-frontend` [localhost:8080](https://localhost:8080)
- `make port-forward-mongo` (exposes port 27017 for local mongo interactions)
- `make port-forward-mongo-express` [localhost:8082](https://localhost:8082)
