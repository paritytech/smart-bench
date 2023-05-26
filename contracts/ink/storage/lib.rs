#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod storage {
    use ink::storage::Mapping;

    #[ink(storage)]
    #[derive(Default)]
    pub struct Storage {
        balances: Mapping<AccountId, Balance>,
    }

    impl Storage {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                balances: Mapping::default()
            }
        }

        #[ink(message)]
        pub fn read(&self, account: AccountId, count: u32) {
            for _ in 0..count {
                let _ = self.balances.get(account);
            }
        }

        #[ink(message)]
        pub fn write(&mut self, account: AccountId, count: u32) {
            for _ in 0..count {
                self.balances.insert(account, &1_000_000);
            }
        }

        #[ink(message)]
        pub fn read_write(&mut self, account: AccountId, count: u32) {
            for _ in 0..count {
                let x = self.balances.get(account).unwrap_or(0);
                self.balances.insert(account, &(x + 1));
            }
        }

        #[ink(message)]
        pub fn read_write_raw(&mut self, account: AccountId, count: u32) {
            let account_key = ink::primitives::KeyComposer::from_bytes(account.as_ref());
            for _ in 0..count {
                let x = ink::env::get_contract_storage(&account_key).unwrap().unwrap_or(0);
                ink::env::set_contract_storage(&account_key, &(x + 1));
            }
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let storage = Storage::default();
            assert_eq!(storage.get(), false);
        }

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            let mut storage = Storage::new(false);
            assert_eq!(storage.get(), false);
            storage.flip();
            assert_eq!(storage.get(), true);
        }
    }
}
