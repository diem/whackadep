.PHONY: all front

front:
	docker build --pull --rm -f "web-frontend/Dockerfile" -t whackadep-front:latest "web-frontend"
	docker run --rm -it  -p 8080:8080/tcp whackadep-front:latest

all:
	docker-compose build
	docker-compose up
	