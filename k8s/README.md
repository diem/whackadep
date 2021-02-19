# Kubernetes development & deployment flow

## Setup

First install [kind](https://kind.sigs.k8s.io/):

> If you have go (1.11+) and docker installed `GO111MODULE="on" go get sigs.k8s.io/kind@v0.10.0 && kind create cluster` is all you need!

then delete the default cluster:

```
kind delete cluster
```

and create our own cluster with a local repository:

```
make launch-local-kind-cluster
```

## Development

TKTK

## Deployment

TKTK

