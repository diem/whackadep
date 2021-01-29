.PHONY: all fast frontend backend

all:
	docker-compose build
	docker-compose up

fast:
	docker-compose up

restart-backend:
	docker-compose up --detach --build backend

restart-frontend:
	docker-compose up --detach --build frontend
