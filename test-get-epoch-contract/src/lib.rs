use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{
    env, ext_contract, is_promise_success, log, near_bindgen, Balance, Gas, PanicOnDefault,
    PromiseOrValue,
};

#[cfg(target_arch = "wasm32")]
use near_sdk::env::BLOCKCHAIN_INTERFACE;

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc = near_sdk::wee_alloc::WeeAlloc::INIT;

const ONE_NEAR: Balance = 1_000_000_000_000_000_000_000_000;
const NEAR: Balance = ONE_NEAR;
const TGAS: Gas = 1_000_000_000_000;
const NO_DEPOSIT: u128 = 0;

type U128String = U128;

const CONTRACT_VERSION: &str = "0.0.1"; //to test Sputnik V2 remote-upgrade

#[cfg(target_arch = "wasm32")]
const BLOCKCHAIN_INTERFACE_NOT_SET_ERR: &str = "Blockchain interface not set.";
#[cfg(target_arch = "wasm32")]
const GAS_FOR_UPGRADE: u64 = 50 * TGAS;

//contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct TestContract {
    //test state
    pub saved_message: String,
    pub saved_i32: i32,
    //last response received
    pub last_epoch: u64,
}

#[ext_contract(ext_staking_pool)]
pub trait ExtStakingPool {
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128String;

    fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128String;

    fn get_account_total_balance(&self, account_id: AccountId) -> U128String;

    fn deposit(&mut self);

    fn deposit_and_stake(&mut self);

    fn withdraw(&mut self, amount: U128String);
    fn withdraw_all(&mut self);

    fn stake(&mut self, amount: U128String);

    fn unstake(&mut self, amount: U128String);

    fn unstake_all(&mut self);
}

#[ext_contract(ext_self_owner)]
pub trait SelfCallbacks {
    fn on_get_sp_total_balance(&mut self, big_amount: u128, #[callback] total_balance: U128String);
}

#[near_bindgen]
impl TestContract {
    #[init]
    pub fn new() -> Self {
        /* Prevent re-initializations */
        assert!(!env::state_exists(), "This contract is already initialized");
        return Self {
            saved_message: String::from("init"),
            saved_i32: 0,
            last_epoch: env::epoch_height(),
        };
    }

    // ------------------------------
    // test Sputnik V2 remote-upgrade
    // ------------------------------
    /// get version ()
    pub fn get_version(self) -> String {
        CONTRACT_VERSION.into()
    }

    // ------------------------------
    // Main methods
    // ------------------------------
    #[payable]
    pub fn set_message(&mut self, message: String) {
        self.saved_message = message;
    }
    #[payable]
    pub fn set_i32(&mut self, num: i32) {
        self.saved_i32 = num;
    }

    pub fn get_message(&self) -> String {
        return self.saved_message.clone();
    }

    ///Make a request to the dia-gateway smart contract
    pub fn get_epoch_height(&self) -> u64 {
        return env::epoch_height();
    }

    ///Make a request to the dia-gateway smart contract
    pub fn get_block_index(&self) -> u64 {
        return env::block_index();
    }

    // ------------------------------
    //Test u128 as argument type in a callback
    // ------------------------------
    pub fn test_callbacks(&self) -> PromiseOrValue<u128> {
        let big_amount: u128 = u128::MAX;
        //query our current balance (includes staked+unstaked+staking rewards)
        ext_staking_pool::get_account_total_balance(
            String::from("lucio.testnet"),
            //promise params
            &String::from("meta.pool.testnet"),
            NO_DEPOSIT,
            10 * TGAS,
        )
        .then(ext_self_owner::on_get_sp_total_balance(
            big_amount,
            //promise params
            &env::current_account_id(),
            NO_DEPOSIT,
            10 * TGAS,
        ))
        .into()
    }
    //prev-fn continues here
    #[private]
    pub fn on_get_sp_total_balance(
        big_amount: u128,
        #[callback] balance: U128String,
    ) -> U128String {
        log!(
            "is_promise_success:{} big_amount:{} big_amount(nears):{} balance:{}",
            is_promise_success(),
            big_amount,
            big_amount / NEAR,
            balance.0
        );
        return balance;
    }

    /// upgrade code and then call migrate() on the new code, called from Sputnik V2
    #[cfg(target_arch = "wasm32")]
    pub fn upgrade() {
        assert!(env::predecessor_account_id() == "dao.pool.testnet");
        //log!("bytes.length {}",code.unwrap().len() );
        let current_id = env::current_account_id().into_bytes();
        let method_name = "migrate".as_bytes().to_vec();
        let attached_gas = env::prepaid_gas() - env::used_gas() - GAS_FOR_UPGRADE;
        unsafe {
            BLOCKCHAIN_INTERFACE.with(|b| {
                // Load input (new contract code) into register 0
                b.borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .input(0);
                //prepare self-call promise
                let promise_id = b
                    .borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .promise_batch_create(current_id.len() as _, current_id.as_ptr() as _);

                //1st item, upgrade code (takes data from register 0)
                b.borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .promise_batch_action_deploy_contract(promise_id, u64::MAX as _, 0);

                //2nd item, schedule a call to "migrate".- (will execute on the *new code*)
                b.borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .promise_batch_action_function_call(
                        promise_id,
                        method_name.len() as _,
                        method_name.as_ptr() as _,
                        0 as _,
                        0 as _,
                        0 as _,
                        attached_gas,
                    );
            });
        }
    }

    //-----------------
    //-- migration called after code upgrade
    //-----------------
    /// Should only be called by this contract on migration.
    /// This is NOOP implementation. KEEP IT if you haven't changed contract state.
    /// If you have changed state, you need to implement migration from old state (keep the old struct with different name to deserialize it first).
    /// After migrate goes live on MainNet, return this implementation for next updates.
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "ERR_NOT_ALLOWED"
        );
        //read old state (old structure with different name)
        let old: TestContract = env::state_read().expect("ERR_CONTRACT_IS_NOT_INITIALIZED");
        //Create the new contract using the data from the old contract.
        let new: TestContract = old;
        return new; //return new struct, will be stored as contract state
    }
}

// ------------------------------
// Unit tests
// ------------------------------

#[cfg(test)]
mod tests {
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    /// Set the contract context
    pub fn initialize() {
        let context = get_context(String::from("client.testnet"), 10);
        testing_env!(context);
    }

    /// Defines the context for the contract
    fn get_context(predecessor_account_id: String, storage_usage: u64) -> VMContext {
        VMContext {
            current_account_id: "contract.testnet".to_string(),
            signer_account_id: "alice.testnet".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 19,
        }
    }

    ///Test get_id and set_id methods
    #[test]
    fn test_id() {
        initialize();
        /* Initialize contract */
        let mut contract = super::TestContract::new();
        let msg = String::from("test string");
        contract.set_message(msg.clone());
        assert_eq!(
            contract.get_message(),
            msg.clone(),
            "Contract message is different from the expected"
        );
    }
}
