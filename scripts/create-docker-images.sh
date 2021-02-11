#!/bin/bash

build_dirs=( "web-backend" "web-frontend")
for build in ${build_dirs[@]}; do 
  pushd ${build}
  docker build -t ${build} .
  popd
done;
