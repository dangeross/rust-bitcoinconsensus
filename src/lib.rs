// Copyright 2018 Tamas Blummer
// SPDX-License-Identifier: Apache-2.0

//! This project builds the `libbitcoinconsensus` library from Bitcoin's C++
//! sources using Cargo and provides Rust bindings to its API.
//!
//! Quoting from [`bitcoin/doc/shared-libraries.md`]:
//!
//! > The purpose of this library is to make the verification functionality that is critical to
//! > Bitcoin's consensus available to other applications, e.g. to language bindings.
//!
//! And that is exactly what this library is, the Rust bindings to `bitcoinconsensus`.
//!
//! [`bitcoin/doc/shared-libraries.md`]: <https://github.com/bitcoin/bitcoin/blob/master/doc/shared-libraries.md>

mod types;

use core::fmt;

use crate::types::c_uint;

/// Do not enable any verification.
pub const VERIFY_NONE: c_uint = 0;
/// Evaluate P2SH (BIP16) subscripts.
pub const VERIFY_P2SH: c_uint = 1 << 0;
/// Enforce strict DER (BIP66) compliance.
pub const VERIFY_DERSIG: c_uint = 1 << 2;
/// Enforce NULLDUMMY (BIP147).
pub const VERIFY_NULLDUMMY: c_uint = 1 << 4;
/// Enable CHECKLOCKTIMEVERIFY (BIP65).
pub const VERIFY_CHECKLOCKTIMEVERIFY: c_uint = 1 << 9;
/// Enable CHECKSEQUENCEVERIFY (BIP112).
pub const VERIFY_CHECKSEQUENCEVERIFY: c_uint = 1 << 10;
/// Enable WITNESS (BIP141).
pub const VERIFY_WITNESS: c_uint = 1 << 11;

pub const VERIFY_ALL: c_uint = VERIFY_P2SH
    | VERIFY_DERSIG
    | VERIFY_NULLDUMMY
    | VERIFY_CHECKLOCKTIMEVERIFY
    | VERIFY_CHECKSEQUENCEVERIFY
    | VERIFY_WITNESS;

/// Computes flags for soft fork activation heights on the Bitcoin network.
pub fn height_to_flags(height: u32) -> u32 {
    let mut flag = VERIFY_NONE;

    if height >= 173805 {
        flag |= VERIFY_P2SH;
    }
    if height >= 363725 {
        flag |= VERIFY_DERSIG;
    }
    if height >= 388381 {
        flag |= VERIFY_CHECKLOCKTIMEVERIFY;
    }
    if height >= 419328 {
        flag |= VERIFY_CHECKSEQUENCEVERIFY;
    }
    if height >= 481824 {
        flag |= VERIFY_NULLDUMMY | VERIFY_WITNESS
    }

    flag
}

/// Returns `libbitcoinconsensus` version.
pub fn version() -> u32 { unsafe { ffi::bitcoinconsensus_version() as u32 } }

/// Verifies a single spend (input) of a Bitcoin transaction.
///
/// Note that amount will only be checked for Segwit transactions.
///
/// # Arguments
///
///  * `spend_output`: A Bitcoin transaction output script to be spent, serialized in Bitcoin's on wire format.
///  * `amount`: The spent output amount in satoshis.
///  * `spending_transaction`: The spending Bitcoin transaction, serialized in Bitcoin's on wire format.
///  * `input_index`: The index of the input within spending_transaction.
///
/// # Examples
///
/// The (randomly choosen) Bitcoin transaction
///
///  `aca326a724eda9a461c10a876534ecd5ae7b27f10f26c3862fb996f80ea2d45d`
///
/// spends one input, that is the first output of
///
///  `95da344585fcf2e5f7d6cbf2c3df2dcce84f9196f7a7bb901a43275cd6eb7c3f`
///
/// The spending transaction serialized is:
///
///  `spending = 02000000013f7cebd65c27431a90bba7f796914fe8cc2ddfc3f2cbd6f7e5f2fc854534da95000000006b483045022100de1ac3bcdfb0332207c4a91f3832bd2c2915840165f876ab47c5f8996b971c3602201c6c053d750fadde599e6f5c4e1963df0f01fc0d97815e8157e3d59fe09ca30d012103699b464d1d8bc9e47d4fb1cdaa89a1c5783d68363c4dbc4b524ed3d857148617feffffff02836d3c01000000001976a914fc25d6d5c94003bf5b0c7b640a248e2c637fcfb088ac7ada8202000000001976a914fbed3d9b11183209a57999d54d59f67c019e756c88ac6acb0700`
///
/// The script of the first output of the spent transaction is:
///
///  `spent = 76a9144bfbaf6afb76cc5771bc6404810d1cc041a6933988ac`
///
/// The (pseudo code) call: `verify(spent, 630482530, spending, 0)` should return `Ok(())`.
///
/// **Note** since the spent amount will only be checked for Segwit transactions and the above
/// example is not segwit, `verify` will succeed with any amount.
pub fn verify(
    spent_output: &[u8],
    amount: u64,
    spending_transaction: &[u8],
    input_index: usize,
) -> Result<(), Error> {
    verify_with_flags(spent_output, amount, spending_transaction, input_index, VERIFY_ALL)
}

/// Same as verify but with flags that turn past soft fork features on or off.
pub fn verify_with_flags(
    spent_output_script: &[u8],
    amount: u64,
    spending_transaction: &[u8],
    input_index: usize,
    flags: u32,
) -> Result<(), Error> {
    unsafe {
        let mut error = Error::ERR_SCRIPT;

        let ret = ffi::bitcoinconsensus_verify_script_with_amount(
            spent_output_script.as_ptr(),
            spent_output_script.len() as c_uint,
            amount,
            spending_transaction.as_ptr(),
            spending_transaction.len() as c_uint,
            input_index as c_uint,
            flags as c_uint,
            &mut error,
        );
        if ret != 1 {
            Err(error)
        } else {
            Ok(())
        }
    }
}

pub mod ffi {
    use crate::types::{c_int, c_uchar, c_uint};
    use crate::Error;

    extern "C" {
        /// Returns `libbitcoinconsensus` version.
        pub fn bitcoinconsensus_version() -> c_int;

        /// Verifies that the transaction input correctly spends the previous
        /// output, considering any additional constraints specified by flags.
        pub fn bitcoinconsensus_verify_script_with_amount(
            script_pubkey: *const c_uchar,
            script_pubkeylen: c_uint,
            amount: u64,
            tx_to: *const c_uchar,
            tx_tolen: c_uint,
            n_in: c_uint,
            flags: c_uint,
            err: *mut Error,
        ) -> c_int;
    }
}

/// Errors returned by [`libbitcoinconsensus`].
///
/// The error variant identifiers mimic those from `libbitcoinconsensus`.
///
/// [`libbitcoinconsensus`]: <https://github.com/bitcoin/bitcoin/blob/master/doc/shared-libraries.md#errors>
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum Error {
    /// Default value, passed to `libbitcoinconsensus` as a return parameter.
    ERR_SCRIPT = 0,
    /// An invalid index for `txTo`.
    ERR_TX_INDEX,
    /// `txToLen` did not match with the size of `txTo`.
    ERR_TX_SIZE_MISMATCH,
    /// An error deserializing `txTo`.
    ERR_TX_DESERIALIZE,
    /// Input amount is required if WITNESS is used.
    ERR_AMOUNT_REQUIRED,
    /// Script verification `flags` are invalid (i.e. not part of the libconsensus interface).
    ERR_INVALID_FLAGS,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;

        let s = match *self {
            ERR_SCRIPT => "error value was not set (value still 0)",
            ERR_TX_INDEX => "an invalid index for txTo",
            ERR_TX_SIZE_MISMATCH => "txToLen did not match with the size of txTo",
            ERR_TX_DESERIALIZE => "an error deserializing txTo",
            ERR_AMOUNT_REQUIRED => "input amount is required if WITNESS is used",
            ERR_INVALID_FLAGS => "script verification flags are invalid",
        };
        f.write_str(s)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use self::Error::*;

        match *self {
            ERR_SCRIPT | ERR_TX_INDEX | ERR_TX_SIZE_MISMATCH | ERR_TX_DESERIALIZE
            | ERR_AMOUNT_REQUIRED | ERR_INVALID_FLAGS => None,
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate rustc_serialize as serialize;

    use self::serialize::hex::FromHex;
    use super::*;

    #[test]
    fn bitcoinconsensus_test() {
        // a random old-style transaction from the blockchain
        verify_test (
            "76a9144bfbaf6afb76cc5771bc6404810d1cc041a6933988ac",
            "02000000013f7cebd65c27431a90bba7f796914fe8cc2ddfc3f2cbd6f7e5f2fc854534da95000000006b483045022100de1ac3bcdfb0332207c4a91f3832bd2c2915840165f876ab47c5f8996b971c3602201c6c053d750fadde599e6f5c4e1963df0f01fc0d97815e8157e3d59fe09ca30d012103699b464d1d8bc9e47d4fb1cdaa89a1c5783d68363c4dbc4b524ed3d857148617feffffff02836d3c01000000001976a914fc25d6d5c94003bf5b0c7b640a248e2c637fcfb088ac7ada8202000000001976a914fbed3d9b11183209a57999d54d59f67c019e756c88ac6acb0700",
            0, 0
        ).unwrap();

        // a random segwit transaction from the blockchain using P2SH
        verify_test (
            "a91434c06f8c87e355e123bdc6dda4ffabc64b6989ef87",
            "01000000000101d9fd94d0ff0026d307c994d0003180a5f248146efb6371d040c5973f5f66d9df0400000017160014b31b31a6cb654cfab3c50567bcf124f48a0beaecffffffff012cbd1c000000000017a914233b74bf0823fa58bbbd26dfc3bb4ae715547167870247304402206f60569cac136c114a58aedd80f6fa1c51b49093e7af883e605c212bdafcd8d202200e91a55f408a021ad2631bc29a67bd6915b2d7e9ef0265627eabd7f7234455f6012103e7e802f50344303c76d12c089c8724c1b230e3b745693bbe16aad536293d15e300000000",
            1900000, 0
        ).unwrap();

        // a random segwit transaction from the blockchain using native segwit
        verify_test(
            "0020701a8d401c84fb13e6baf169d59684e17abd9fa216c8cc5b9fc63d622ff8c58d",
            "010000000001011f97548fbbe7a0db7588a66e18d803d0089315aa7d4cc28360b6ec50ef36718a0100000000ffffffff02df1776000000000017a9146c002a686959067f4866b8fb493ad7970290ab728757d29f0000000000220020701a8d401c84fb13e6baf169d59684e17abd9fa216c8cc5b9fc63d622ff8c58d04004730440220565d170eed95ff95027a69b313758450ba84a01224e1f7f130dda46e94d13f8602207bdd20e307f062594022f12ed5017bbf4a055a06aea91c10110a0e3bb23117fc014730440220647d2dc5b15f60bc37dc42618a370b2a1490293f9e5c8464f53ec4fe1dfe067302203598773895b4b16d37485cbe21b337f4e4b650739880098c592553add7dd4355016952210375e00eb72e29da82b89367947f29ef34afb75e8654f6ea368e0acdfd92976b7c2103a1b26313f430c4b15bb1fdce663207659d8cac749a0e53d70eff01874496feff2103c96d495bfdd5ba4145e3e046fee45e84a8a48ad05bd8dbb395c011a32cf9f88053ae00000000",
            18393430 , 0
        ).unwrap();

        // a random old-style transaction from the blockchain - WITH WRONG SIGNATURE for the address
        assert!(verify_test (
            "76a9144bfbaf6afb76cc5771bc6404810d1cc041a6933988ff",
            "02000000013f7cebd65c27431a90bba7f796914fe8cc2ddfc3f2cbd6f7e5f2fc854534da95000000006b483045022100de1ac3bcdfb0332207c4a91f3832bd2c2915840165f876ab47c5f8996b971c3602201c6c053d750fadde599e6f5c4e1963df0f01fc0d97815e8157e3d59fe09ca30d012103699b464d1d8bc9e47d4fb1cdaa89a1c5783d68363c4dbc4b524ed3d857148617feffffff02836d3c01000000001976a914fc25d6d5c94003bf5b0c7b640a248e2c637fcfb088ac7ada8202000000001976a914fbed3d9b11183209a57999d54d59f67c019e756c88ac6acb0700",
            0, 0
        ).is_err());

        // a random segwit transaction from the blockchain using P2SH - WITH WRONG AMOUNT
        assert!(verify_test (
            "a91434c06f8c87e355e123bdc6dda4ffabc64b6989ef87",
            "01000000000101d9fd94d0ff0026d307c994d0003180a5f248146efb6371d040c5973f5f66d9df0400000017160014b31b31a6cb654cfab3c50567bcf124f48a0beaecffffffff012cbd1c000000000017a914233b74bf0823fa58bbbd26dfc3bb4ae715547167870247304402206f60569cac136c114a58aedd80f6fa1c51b49093e7af883e605c212bdafcd8d202200e91a55f408a021ad2631bc29a67bd6915b2d7e9ef0265627eabd7f7234455f6012103e7e802f50344303c76d12c089c8724c1b230e3b745693bbe16aad536293d15e300000000",
            900000, 0).is_err());

        // a random segwit transaction from the blockchain using native segwit - WITH WRONG SEGWIT
        assert!(verify_test(
            "0020701a8d401c84fb13e6baf169d59684e17abd9fa216c8cc5b9fc63d622ff8c58f",
            "010000000001011f97548fbbe7a0db7588a66e18d803d0089315aa7d4cc28360b6ec50ef36718a0100000000ffffffff02df1776000000000017a9146c002a686959067f4866b8fb493ad7970290ab728757d29f0000000000220020701a8d401c84fb13e6baf169d59684e17abd9fa216c8cc5b9fc63d622ff8c58d04004730440220565d170eed95ff95027a69b313758450ba84a01224e1f7f130dda46e94d13f8602207bdd20e307f062594022f12ed5017bbf4a055a06aea91c10110a0e3bb23117fc014730440220647d2dc5b15f60bc37dc42618a370b2a1490293f9e5c8464f53ec4fe1dfe067302203598773895b4b16d37485cbe21b337f4e4b650739880098c592553add7dd4355016952210375e00eb72e29da82b89367947f29ef34afb75e8654f6ea368e0acdfd92976b7c2103a1b26313f430c4b15bb1fdce663207659d8cac749a0e53d70eff01874496feff2103c96d495bfdd5ba4145e3e046fee45e84a8a48ad05bd8dbb395c011a32cf9f88053ae00000000",
            18393430 , 0
        ).is_err());
    }

    fn verify_test(spent: &str, spending: &str, amount: u64, input: usize) -> Result<(), Error> {
        verify(
            spent.from_hex().unwrap().as_slice(),
            amount,
            spending.from_hex().unwrap().as_slice(),
            input,
        )
    }

    #[test]
    fn invalid_flags_test() { verify_with_flags(&[], 0, &[], 0, VERIFY_ALL + 1).unwrap_err(); }
}
