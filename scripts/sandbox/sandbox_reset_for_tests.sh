#!/bin/bash

# run before each test

# reset sandbox

echo "start script"
sandbox reset dev -v
echo "after reset"
sh ./fund_accounts_sandbox.sh
