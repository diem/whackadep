.PHONY: all fast frontend backend

all:
	docker-compose build
	docker-compose up

fast:
	docker-compose up

frontend:
	cd web-frontend && PROXY="http://localhost:8000" yarn serve

backend:
	# volume to persist data between runs (like target or diem_repo)	
	docker volume create whackadep-backend
	docker volume create whackadep-backend-cargo
	# build
	docker build --pull --rm -f "web-backend/Dockerfile" -t whackadep-backend:latest "web-backend"
	# run with port 8081 and
	# github access token from env var GITHUB_TOKEN
	RUST_BACKTRACE=1 MONGODB_URI="mongodb://root:password@localhost:27017"
	docker run \
		--rm \
		--mount source=whackadep-backend-cargo,target=/cargo \
		--mount source=whackadep-backend,target=/app \
		-p 8081:8081 \
		--env-file=web-backend/github_token \
		--env RUST_BACKTRACE=1 \
		--env MONGODB_URI="mongodb://root:password@localhost:27017" \
		-it \
		whackadep-backend:latest

backend-no-container:
	# you need to call this with a GITHUB_TOKEN
	cd web-backend && RUST_LOG=info RUST_BACKTRACE=1 MONGODB_URI="mongodb://root:password@localhost:27017" cargo run

database:
	cd db && docker-compose up

restart-backend:
	docker-compose up --detach --build backend
