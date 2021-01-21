.PHONY: all fast

all:
	docker-compose build
	docker-compose up

fast:
	docker-compose up
	