#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Env, String, Vec, Map, Address,
};

/// Satu entri audit trail transaksi yang tersimpan on-chain.
/// Field yang disimpan dibatasi ketat untuk menjaga privasi:
/// TIDAK ADA data sensitif (nama pembeli, rincian item, dst).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TransactionRecord {
    /// ID merchant (referensi ke sistem backend)
    pub merchant_id: String,
    /// Nominal transaksi (dalam unit stablecoin / IDR)
    pub amount: i128,
    /// Unix timestamp (epoch seconds) saat transaksi dikonfirmasi
    pub timestamp: u64,
    /// Hash referensi unik per transaksi (Midtrans order_id atau internal ID)
    /// Dipakai untuk idempotency — mencegah duplikat on-chain
    pub tx_ref_hash: String,
}

#[contract]
pub struct LedgerContract;

#[contractimpl]
impl LedgerContract {
    /// Simpan ringkasan transaksi ke ledger on-chain.
    /// Fungsi ini append-only — tidak ada fungsi update/delete.
    ///
    /// Idempotency: jika tx_ref_hash sudah pernah direkam,
    /// fungsi ini akan panic untuk mencegah data duplikat.
    ///
    /// # Arguments
    /// * `admin`       - Address operator backend yang memanggil fungsi ini (perlu require_auth)
    /// * `merchant_id` - ID merchant (dari database backend)
    /// * `amount`      - Nominal dalam satuan terkecil IDR (sen / unit stablecoin * 100)
    /// * `timestamp`   - Unix timestamp epoch seconds
    /// * `tx_ref_hash` - Hash unik per transaksi (order_id atau SHA-256 reference)
    ///
    /// # Returns
    /// Index record yang baru dibuat
    pub fn record_transaction(
        env: Env,
        admin: Address,
        merchant_id: String,
        amount: i128,
        timestamp: u64,
        tx_ref_hash: String,
    ) -> u64 {
        // Otentikasi
        admin.require_auth();

        // Cek idempotency — apakah tx_ref_hash sudah pernah direkam?
        let hash_key = symbol_short!("H");
        let mut hash_index: Map<String, u64> = env
            .storage()
            .persistent()
            .get(&hash_key)
            .unwrap_or(Map::new(&env));

        if hash_index.contains_key(tx_ref_hash.clone()) {
            panic!("Transaction already recorded");
        }

        // Dapatkan counter record saat ini
        let count_key = symbol_short!("COUNT");
        let current_count: u64 = env
            .storage()
            .persistent()
            .get(&count_key)
            .unwrap_or(0u64);

        let new_index = current_count;

        // Buat record baru
        let record = TransactionRecord {
            merchant_id: merchant_id.clone(),
            amount,
            timestamp,
            tx_ref_hash: tx_ref_hash.clone(),
        };

        // Simpan record dengan key berbasis index global
        let record_key = (symbol_short!("REC"), new_index);
        env.storage().persistent().set(&record_key, &record);

        // Update hash index untuk idempotency
        hash_index.set(tx_ref_hash, new_index);
        env.storage().persistent().set(&hash_key, &hash_index);

        // Increment global counter
        env.storage().persistent().set(&count_key, &(new_index + 1));

        // Update indeks per-merchant
        let merchant_key = (symbol_short!("M_IDX"), merchant_id);
        let mut merchant_indices: Vec<u64> = env
            .storage()
            .persistent()
            .get(&merchant_key)
            .unwrap_or(Vec::new(&env));
        merchant_indices.push_back(new_index);
        env.storage().persistent().set(&merchant_key, &merchant_indices);

        // Emit event untuk memudahkan indexing off-chain
        env.events().publish(
            (symbol_short!("tx_rec"), symbol_short!("new")),
            new_index,
        );

        new_index
    }

    /// Ambil total jumlah record yang sudah tersimpan.
    pub fn get_record_count(env: Env) -> u64 {
        let count_key = symbol_short!("COUNT");
        env.storage().persistent().get(&count_key).unwrap_or(0u64)
    }

    /// Ambil satu record berdasarkan index.
    /// Mengembalikan None jika index tidak valid.
    pub fn get_record(env: Env, index: u64) -> Option<TransactionRecord> {
        let record_key = (symbol_short!("REC"), index);
        env.storage().persistent().get(&record_key)
    }

    /// Cek apakah sebuah tx_ref_hash sudah pernah direkam (idempotency check).
    pub fn is_recorded(env: Env, tx_ref_hash: String) -> bool {
        let hash_key = symbol_short!("H");
        let hash_index: Map<String, u64> = env
            .storage()
            .persistent()
            .get(&hash_key)
            .unwrap_or(Map::new(&env));
        hash_index.contains_key(tx_ref_hash)
    }

    /// Mengembalikan daftar transaksi per merchant pada rentang timestamp tertentu
    pub fn get_merchant_history(
        env: Env,
        merchant_id: String,
        from: u64,
        to: u64,
    ) -> Vec<TransactionRecord> {
        let merchant_key = (symbol_short!("M_IDX"), merchant_id);
        let merchant_indices: Vec<u64> = env
            .storage()
            .persistent()
            .get(&merchant_key)
            .unwrap_or(Vec::new(&env));
        
        let mut result = Vec::new(&env);
        for index in merchant_indices.iter() {
            let record_key = (symbol_short!("REC"), index);
            if let Some(record) = env.storage().persistent().get::<_, TransactionRecord>(&record_key) {
                if record.timestamp >= from && record.timestamp <= to {
                    result.push_back(record);
                }
            }
        }
        result
    }

    /// Mengembalikan agregasi total omzet (amount) per merchant pada rentang timestamp tertentu
    pub fn get_total_volume(
        env: Env,
        merchant_id: String,
        from: u64,
        to: u64,
    ) -> i128 {
        let history = Self::get_merchant_history(env, merchant_id, from, to);
        let mut total = 0i128;
        for record in history.iter() {
            total += record.amount;
        }
        total
    }
}

#[cfg(test)]
mod test {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_record_transaction_basic() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(LedgerContract, ());
        let client = LedgerContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let merchant_id = soroban_sdk::String::from_str(&env, "merchant-uuid-123");
        let tx_ref_hash = soroban_sdk::String::from_str(&env, "ZETAFI-ABCD1234-1720000000000");

        let index = client.record_transaction(
            &admin,
            &merchant_id,
            &150000i128, // Rp 1.500,00
            &1720000000u64,
            &tx_ref_hash,
        );

        assert_eq!(index, 0u64);
        assert_eq!(client.get_record_count(), 1u64);

        let record = client.get_record(&0u64).unwrap();
        assert_eq!(record.amount, 150000i128);
        assert_eq!(record.timestamp, 1720000000u64);
    }

    #[test]
    #[should_panic(expected = "Transaction already recorded")]
    fn test_idempotency_panic() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(LedgerContract, ());
        let client = LedgerContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let merchant_id = soroban_sdk::String::from_str(&env, "merchant-uuid-123");
        let tx_ref_hash = soroban_sdk::String::from_str(&env, "SAME-HASH-001");

        // Submit pertama
        client.record_transaction(
            &admin, &merchant_id, &100000i128, &1720000000u64, &tx_ref_hash,
        );

        // Submit kedua dengan hash yang sama harus panic
        client.record_transaction(
            &admin, &merchant_id, &100000i128, &1720000001u64, &tx_ref_hash,
        );
    }

    #[test]
    fn test_merchant_history_and_volume() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(LedgerContract, ());
        let client = LedgerContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let merchant_id = soroban_sdk::String::from_str(&env, "merchant-uuid-123");
        let other_merchant = soroban_sdk::String::from_str(&env, "other-merchant");

        // Insert records for merchant_id
        client.record_transaction(&admin, &merchant_id, &1000, &100, &soroban_sdk::String::from_str(&env, "h1"));
        client.record_transaction(&admin, &merchant_id, &2000, &150, &soroban_sdk::String::from_str(&env, "h2"));
        client.record_transaction(&admin, &merchant_id, &3000, &200, &soroban_sdk::String::from_str(&env, "h3"));

        // Insert record for another merchant
        client.record_transaction(&admin, &other_merchant, &5000, &150, &soroban_sdk::String::from_str(&env, "h4"));

        // Query history for merchant_id between timestamp 100 and 150
        let history = client.get_merchant_history(&merchant_id, &100, &150);
        assert_eq!(history.len(), 2);

        // Check total volume for same period
        let volume = client.get_total_volume(&merchant_id, &100, &150);
        assert_eq!(volume, 3000); // 1000 + 2000

        // Query full history
        let full_volume = client.get_total_volume(&merchant_id, &0, &999);
        assert_eq!(full_volume, 6000);
    }
}
