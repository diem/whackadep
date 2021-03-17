# Kubernetes development & deployment flow

## Setup

First, install [Docker](https://docs.docker.com/get-docker/).

Then, install [kind](https://kind.sigs.k8s.io/). 
You do not need to use *kind*, but you will have to figure out how to setup a local registry with your local kubernetes instance. 
(The makefile target 'launch-local-kind-cluster' handles this for KIND clusters)

> If you have go (1.11+) and docker installed `GO111MODULE="on" go get sigs.k8s.io/kind@v0.10.0 && kind create cluster` is all you need!


After, that, delete the default cluster:

```
kind delete cluster
```

and re-create your own cluster with the custom local repository:

```
make launch-local-kind-cluster
```

## Development

Set up the cluster:

```
make launch-local-kind-cluster
```

Build the docker images:

```
make create-docker-images
```

Push Docker Images to local registry:

```
make push-docker-images
```

Apply kubernetes manifests to local cluster:

```
make apply-local-k8s
```


### Port Forwarding

Forward various ports for local interaction with the kubernetes cluster.

Run the following `make` targets in separate terminal shells
- `make port-forward-backend` [localhost:8081](https://localhost:8081)
- `make port-forward-frontend` [localhost:8080](https://localhost:8080)
- `make port-forward-mongo` (exposes port 27017 for local mongo interactions)
- `make port-forward-mongo-express` [localhost:8082](https://localhost:8082)

## Deployment

TKTK


