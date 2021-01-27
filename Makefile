.PHONY: all fast frontend backend

all:
	docker-compose build
	docker-compose up

fast:
	docker-compose up

frontend:
	cd web-frontend && PROXY="http://localhost:8000" yarn serve

backend:
	cd web-backend && RUST_LOG=info RUST_BACKTRACE=1 MONGODB_URI="mongodb://root:password@localhost:27017" cargo run

database:
	cd db && docker-compose up
