#!/bin/bash
# sometimes the indexer gets stuck
# so far only completely resetting the environment fixes it

docker system prune -a -f --volumes
docker system prune -a

docker rmi sandbox_algod -f
docker rmi sandbox_indexer -f
docker rmi postgres -f

# start (dev mode)
sandbox up dev -v
