#!/bin/bash

repo="localhost:5000"
build_dirs=( "web-backend" "web-frontend")
for build in ${build_dirs[@]}; do 
  pushd ${build}
  docker build -t "${repo}/${build}" .
  popd
done;
