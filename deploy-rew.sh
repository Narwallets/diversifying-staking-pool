set -e
NETWORK=testnet
OWNER=lucio.$NETWORK
MASTER_ACC=rewards.$NETWORK
CONTRACT_ACC=test.$MASTER_ACC

export NODE_ENV=$NETWORK

## delete acc
#echo "Delete $CONTRACT_ACC? are you sure? Ctrl-C to cancel"
#read input
#near delete $CONTRACT_ACC $MASTER_ACC
#near create-account $CONTRACT_ACC --masterAccount $MASTER_ACC
near deploy $CONTRACT_ACC ./res/rewards_register.wasm --accountId $OWNER
#near call $CONTRACT_ACC new {\"owner_account_id\":\"$OWNER\"} --accountId $OWNER
#meta new { owner_account_id:$OWNER, treasury_account_id:treasury.$CONTRACT_ACC, operator_account_id:operator.$CONTRACT_ACC} --accountId $MASTER_ACC
## set params@meta set_params
#meta default_pools_testnet


## redeploy code only
#meta deploy ./res/metapool.wasm  --accountId $MASTER_ACC

#save last deployment  (to be able to recover state/tokens)
#cp ./res/metapool.wasm ./res/metapool.`date +%F.%T`.wasm
#date +%F.%T
