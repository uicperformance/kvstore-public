#!/usr/bin/env bash

touch "$1"
while [ -f "$1" ]; do sleep 0.1; done
