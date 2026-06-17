#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct LedgerContract;

#[contractimpl]
impl LedgerContract {
    pub fn hello(env: Env) {}
}
