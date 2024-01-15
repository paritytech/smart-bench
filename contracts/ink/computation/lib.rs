#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod computation {
    #[ink(storage)]
    pub struct Computation {}

    impl Computation {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message)]
        pub fn triangle_number(&self, n: i32) -> i64 {
            (1..=n as i64).fold(0, |sum, x| sum.wrapping_add(x))
        }

        #[ink(message)]
        pub fn odd_product(&self, n: i32) -> i64 {
            (1..=n as i64).fold(1, |prod, x| prod.wrapping_mul(2 * x - 1))
        }
    }
}
