.PHONY: all fast frontend backend database restart-frontend restart-backend doc

#
# everything in this section uses the main docker-compose.yml file
#

# run everything for dev
all:
	docker-compose build
	docker-compose up

# run everything for dev without re-building (if you know nothing has changed since last time, this is faster)
fast:
	docker-compose up

# after `make all`, you can run this to rebuild and restart the backend
restart-backend:
	docker-compose up --detach --build backend

# after `make all`, you can run this to rebuild and restart the frontend
restart-frontend:
	docker-compose up --detach --build frontend

# to SSH into the backend server
ssh-backend:
	docker exec -it backend /bin/bash

# to SSH into the frontend server
ssh-frontend:
	docker exec -it frontend /bin/bash


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
