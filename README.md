# ZetaFi — `contracts/`

Smart contract **Soroban (Rust)** untuk audit trail on-chain ZetaFi. Contract ini murni pencatatan (append-only ledger) — tidak ada logika bisnis (stok, harga, keuntungan) di sini, itu semua tetap di `api/`.

Untuk konteks produk lengkap, baca `README.md` di root monorepo. Untuk instruksi kerja AI coding agent khusus repo ini, baca `CLAUDE.md` di folder ini.

Reference belajar: *RiseIn — APAC Stellar Hackathon, modul Soroban Rust* — kontrak pada proyek ini mengikuti pola dasar yang sama: `#![no_std]`, `soroban_sdk`, dikompilasi ke target `wasm32-unknown-unknown`.

---

## 1. `ledger` Contract

Tujuan: mencatat setiap transaksi yang sudah dikonfirmasi (QRIS maupun manual) sebagai entri *append-only* yang tidak bisa diubah atau dihapus.

### Fungsi Inti

| Fungsi | Akses | Fungsi |
|---|---|---|
| `record_transaction(env, merchant_id, amount, timestamp, tx_ref_hash)` | Hanya address backend yang diotorisasi (`require_auth`) | Mencatat satu transaksi. Dipanggil sekali per transaksi yang berhasil dikonfirmasi. Menolak `tx_ref_hash` yang sudah pernah tercatat (idempotency). |
| `get_merchant_history(env, merchant_id, from, to)` | Read-only, publik | Daftar transaksi dalam rentang waktu tertentu — untuk rekonsiliasi/audit eksternal. |
| `get_total_volume(env, merchant_id, period)` | Read-only, publik | Agregasi cepat total omzet dalam periode tertentu. |

### Yang Tersimpan On-Chain

Hanya: `merchant_id`, `amount` (representasi stablecoin), `timestamp`, `tx_ref_hash`. **Tidak ada** rincian item, nama pembeli, atau data bisnis sensitif lainnya — itu semua tetap di Postgres milik `api/`.

---

## 2. Pertimbangan Desain Kontrak

- **Privasi data**: detail transaksi granular tetap di Postgres off-chain. Yang ditulis on-chain cukup untuk verifikasi omzet tanpa membocorkan detail bisnis sensitif.
- **Role-based authorization**: hanya backend service account yang punya izin invoke `record_transaction`, untuk mencegah data palsu disuntikkan langsung ke chain.
- **Idempotency**: setiap job on-chain writer menyertakan `tx_ref_hash` unik dari transaksi Postgres. Kalau job dijalankan ulang (retry), contract harus menolak hash yang sudah pernah direkam.
- **Upgradability**: gunakan pola contract proxy/upgrade Soroban jika diperlukan revisi logic di kemudian hari.
- **Testing**: jalankan `cargo test` dengan `soroban_sdk::testutils` sebelum build WASM dan sebelum deploy ke testnet/mainnet.

---

## 3. Struktur Folder

```
contracts/
├── ledger/
│   ├── src/lib.rs           # record_transaction, get_merchant_history, get_total_volume
│   └── Cargo.toml
└── Cargo.toml                # workspace root
```

---

## 4. Build & Deploy

```bash
# Build contract ke WASM
cd contracts/ledger
cargo build --target wasm32-unknown-unknown --release

# Optimasi WASM (opsional, soroban-cli)
soroban contract optimize --wasm target/wasm32-unknown-unknown/release/ledger.wasm

# Deploy ke testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ledger.wasm \
  --source <ACCOUNT_SECRET> \
  --network testnet
```

Setelah deploy, **update `LEDGER_CONTRACT_ID`** di `api/.env` dan `packages/stellar-config` — kalau tidak, backend akan terus menulis ke contract lama.

---

## 5. Testing

```bash
cd contracts/ledger
cargo test
```

Test wajib mencakup: penolakan invoke tanpa auth yang benar, penolakan `tx_ref_hash` duplikat, dan kebenaran agregasi `get_total_volume`.
