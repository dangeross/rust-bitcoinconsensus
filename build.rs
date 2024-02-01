extern crate cc;

use std::env;

fn main() {
    // Check whether we can use 64-bit compilation
    let use_64bit_compilation = if env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap() == "64" {
        let check = cc::Build::new()
            .file("depend/check_uint128_t.c")
            .cargo_metadata(false)
            .try_compile("check_uint128_t")
            .is_ok();
        if !check {
            println!("cargo:warning=Compiling in 32-bit mode on a 64-bit architecture due to lack of uint128_t support.");
        }
        check
    } else {
        false
    };
    let target = env::var("TARGET").expect("TARGET was not set");
    let is_big_endian: bool = env::var("CARGO_CFG_TARGET_ENDIAN").expect("No endian is set") == "big";
    let mut base_config = cc::Build::new();
    base_config
        .include("depend/bitcoin/src/secp256k1/include")
        .define("__STDC_FORMAT_MACROS", None)
        .flag_if_supported("-Wno-implicit-fallthrough");

    if target.contains("windows") {
        base_config.define("WIN32", "1");
    }

    let mut secp_config = base_config.clone();
    let mut consensus_config = base_config;

    // **Secp256k1**
    if !cfg!(feature = "external-secp") {
        secp_config
            .include("depend/bitcoin/src/secp256k1")
            .include("depend/bitcoin/src/secp256k1/src")
            .flag_if_supported("-Wno-unused-function") // some ecmult stuff is defined but not used upstream
            .define("SECP256K1_BUILD", "1")
            // Bitcoin core defines libsecp to *not* use libgmp.
            .define("USE_NUM_NONE", "1")
            .define("USE_FIELD_INV_BUILTIN", "1")
            .define("USE_SCALAR_INV_BUILTIN", "1")
            // Technically libconsensus doesn't require the recovery feautre, but `pubkey.cpp` does.
            .define("ENABLE_MODULE_RECOVERY", "1")
            .define("ECMULT_WINDOW_SIZE", "15")
            .define("ECMULT_GEN_PREC_BITS", "4")
            .define("ENABLE_MODULE_SCHNORRSIG", "1")
            .define("ENABLE_MODULE_EXTRAKEYS", "1")
            // The actual libsecp256k1 C code.
            .file("depend/bitcoin/src/secp256k1/src/secp256k1.c");

        if is_big_endian {
            secp_config.define("WORDS_BIGENDIAN", "1");
        }
    
        if use_64bit_compilation {
            secp_config
                .define("USE_FIELD_5X52", "1")
                .define("USE_SCALAR_4X64", "1")
                .define("HAVE___INT128", "1");
        } else {
            secp_config.define("USE_FIELD_10X26", "1").define("USE_SCALAR_8X32", "1");
        }

        secp_config.compile("libsecp256k1.a");
    }
    
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS");
    let target_feature = env::var("CARGO_CFG_TARGET_FEATURE");
    if target_os.starts_with("wasi") {
        let wasi_sdk = env::var("WASI_SDK").unwrap_or_else(|_| "/opt/wasi-sdk".to_owned());
        assert!(std::path::Path::new(&wasi_sdk).exists(), "WASI SDK not found at {}", wasi_sdk);
        consensus_config
            .std("c++17")
            .cpp_set_stdlib("c++");
        let wasi_sysroot_lib = match target_feature {
            Ok(target_feature) if target_feature.contains("atomics") => {
                consensus_config
                    .flag("-matomics")
                    .flag("-mbulk-memory");
                "wasm32-wasi-threads"
            }
            _ => "wasm32-wasi",
        };
        println!("cargo:rustc-link-search={wasi_sdk}/share/wasi-sysroot/lib/{wasi_sysroot_lib}");
    } else {
        let tool = consensus_config.get_compiler();

        if tool.is_like_msvc() {
            consensus_config.flag("/std:c++14").flag("/wd4100");
        } else if tool.is_like_clang() || tool.is_like_gnu() {
            consensus_config.flag("-std=c++11").flag("-Wno-unused-parameter");
        }
    }

    consensus_config
        .cpp(true)
        .include("depend/bitcoin/src")
        .include("depend/bitcoin/src/secp256k1/include")
        .file("depend/bitcoin/src/util/strencodings.cpp")
        .file("depend/bitcoin/src/uint256.cpp")
        .file("depend/bitcoin/src/pubkey.cpp")
        .file("depend/bitcoin/src/hash.cpp")
        .file("depend/bitcoin/src/primitives/transaction.cpp")
        .file("depend/bitcoin/src/crypto/ripemd160.cpp")
        .file("depend/bitcoin/src/crypto/sha1.cpp")
        .file("depend/bitcoin/src/crypto/sha256.cpp")
        .file("depend/bitcoin/src/crypto/sha512.cpp")
        .file("depend/bitcoin/src/crypto/hmac_sha512.cpp")
        .file("depend/bitcoin/src/script/bitcoinconsensus.cpp")
        .file("depend/bitcoin/src/script/interpreter.cpp")
        .file("depend/bitcoin/src/script/script.cpp")
        .file("depend/bitcoin/src/script/script_error.cpp")
        .compile("libbitcoinconsensus.a");
}
