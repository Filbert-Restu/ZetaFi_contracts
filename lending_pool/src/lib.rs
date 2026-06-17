#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct LendingPoolContract;

#[contractimpl]
impl LendingPoolContract {
    pub fn hello(env: Env) {}
}
