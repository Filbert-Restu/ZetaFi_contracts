# CLAUDE.md — `contracts/` (Soroban Rust)

Instruksi kerja untuk AI coding agent yang mengedit repo `contracts/`. Baca `CLAUDE.md` root dulu kalau belum. Baca `contracts/README.md` untuk konteks fungsi & alasan desain.

---

## 1. Aturan Inti (Non-Negotiable)

1. **Contract ini tidak menghitung apa pun.** Tidak ada kalkulasi harga, stok, atau keuntungan di Rust. Contract hanya menerima nilai yang sudah final dari backend dan menyimpannya. Kalau ada permintaan menambah "logika" ke contract, cek dulu — kemungkinan besar itu seharusnya di `api/src/services/`.
2. **`record_transaction` wajib `require_auth()` terhadap address backend operator.** Jangan buat fungsi write yang bisa dipanggil siapa saja — ini mencegah data palsu disuntikkan ke ledger.
3. **Idempotency check di dalam contract, bukan cuma di backend.** Simpan `tx_ref_hash` yang sudah pernah masuk (misalnya sebagai key di storage), dan `record_transaction` harus menolak (return error / panic terkontrol) kalau hash yang sama dikirim ulang — jangan hanya mengandalkan idempotency check di sisi `api/`, karena kalau job di-retry dengan kondisi race tertentu, contract adalah baris pertahanan terakhir.
4. **Jangan tambah field baru ke storage tanpa alasan kuat.** Struktur data saat ini sengaja minimal (`merchant_id`, `amount`, `timestamp`, `tx_ref_hash`) untuk alasan privasi. Penambahan field (terutama apa pun yang berbau PII atau detail item) butuh persetujuan eksplisit pemilik proyek — lihat aturan di root `CLAUDE.md` §5.
5. **`#![no_std]` wajib dipertahankan.** Ini requirement Soroban, jangan import crate yang butuh `std` secara tidak sengaja lewat dependency baru.

---

## 2. Konvensi Kode

- Gunakan tipe dari `soroban_sdk` (`Env`, `Address`, `Symbol`, dll), jangan tipe Rust `std` biasa yang tidak kompatibel WASM target ini.
- Storage key pakai pola konsisten (misalnya `Symbol` bernamespace per jenis data: transaksi vs agregat volume) supaya query `get_merchant_history` dan `get_total_volume` efisien dan tidak scan seluruh storage.
- Setiap fungsi publik (`pub fn`) di `lib.rs` butuh doc comment singkat menjelaskan: siapa yang boleh memanggil, apa yang divalidasi, apa yang dikembalikan.

---

## 3. Alur Kerja Wajib Setiap Ada Perubahan

1. Ubah `src/lib.rs`.
2. `cargo test` — **wajib lulus sebelum lanjut**, termasuk test idempotency dan auth rejection.
3. `cargo build --target wasm32-unknown-unknown --release`.
4. (Opsional) `soroban contract optimize`.
5. Deploy ke **testnet** dulu, tidak pernah langsung ke mainnet.
6. Update `LEDGER_CONTRACT_ID` di `api/.env` dan `packages/stellar-config` untuk environment yang relevan.
7. Beri tahu di deskripsi perubahan (commit/PR) kalau ada perubahan bentuk data yang tersimpan — ini mempengaruhi `api/src/services/stellar.service.ts` dan `packages/shared-types`.

---

## 4. Yang Sering Salah (Checklist Sebelum Commit)

- [ ] Tidak ada logika kalkulasi bisnis yang menyusup ke contract.
- [ ] `record_transaction` tetap memvalidasi `require_auth()`.
- [ ] Ada test yang memverifikasi penolakan `tx_ref_hash` duplikat.
- [ ] Tidak ada field baru yang berpotensi PII masuk ke storage tanpa persetujuan.
- [ ] `cargo test` lulus sebelum build WASM.
- [ ] `LEDGER_CONTRACT_ID` diperbarui di semua tempat yang relevan setelah redeploy.
