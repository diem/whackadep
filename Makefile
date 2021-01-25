.PHONY: all fast frontend backend

all:
	docker-compose build
	docker-compose up

fast:
	docker-compose up

frontend:
	cd web-frontend && PROXY="http://localhost:8081" yarn serve

backend:
	cd web-backend && RUST_BACKTRACE=1 MONGODB_URI="mongodb://root:password@localhost:27017" cargo run
