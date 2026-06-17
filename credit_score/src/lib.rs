#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct CreditScoreContract;

#[contractimpl]
impl CreditScoreContract {
    pub fn hello(env: Env) {}
}
