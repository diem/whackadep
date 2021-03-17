#!/bin/bash

build_dirs=( "web-backend" "web-frontend")
container_repo="localhost:5000"

for build in ${build_dirs[@]}; do 
  docker push "${container_repo}/${build}"
done;
