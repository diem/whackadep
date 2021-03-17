.PHONY: all fast frontend backend database restart-frontend restart-backend doc refresh ssh-frontend ssh-backend

# TODO: delete?
args = `arg="$(filter-out $@,$(MAKECMDGOALS))" && echo $${arg:-${1}}`

#
# everything in this section uses the main docker-compose.yml file
#

# run everything for dev
all:
	docker-compose down --volumes
	docker-compose build --no-cache
	docker-compose up

# run everything for dev without re-building (if you know nothing has changed since last time, this is faster)
fast:
	docker-compose up

# after `make all`, you can run this to rebuild and restart the backend
restart-backend:
	docker-compose up --detach --build backend

# after `make all`, you can run this to rebuild and restart the frontend
restart-frontend:
	docker-compose stop frontend
	docker-compose rm -f frontend # needed to remove volume
	docker volume rm whackadep_node_modules # needed to refresh new deps
	docker-compose up --detach --build frontend

# to SSH into the backend server
ssh-backend:
	docker exec -it backend /bin/bash

# to SSH into the frontend server
ssh-frontend:
	docker exec -it frontend /bin/bash

# handy command to trigger a refresh on the API
refresh:
	curl -X GET http://127.0.0.1:8081/refresh

#
# you probably won't need the following command, which run things separately (without the main docker-compose)
#

frontend:
	cd web-frontend && PROXY="http://localhost:8000" yarn serve

backend:
	cd web-backend && RUST_LOG=info RUST_BACKTRACE=1 MONGODB_URI="mongodb://root:password@localhost:27017" cargo run

database:
	cd db && docker-compose up

doc:
	cd web-backend/metrics && cargo doc && open target/doc/metrics/index.html

#
# Kubernetes-specific commands
#

port-forward-backend:
	kubectl port-forward svc/backend 8081:8081

port-forward-frontend:
	kubectl port-forward svc/frontend 8080:8080

port-forward-mongo:
	kubectl port-forward svc/frontend 27017:27017

port-forward-mongo-express:
	kubectl port-forward svc/frontend 8082:8082

launch-local-kind-cluster:
	./scripts/create-kind-cluster-with-local-registry.sh

create-docker-images:
	./scripts/create-docker-images.sh

push-docker-images:
	./scripts/push-docker-images.sh

apply-local-k8s:
	kubectl apply -k k8s/overlay/local
