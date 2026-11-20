fn main() {
    #[cfg(feature = "fsdb-lib")]
    fsdb();
    #[cfg(feature = "shm-lib")]
    shm();
    #[cfg(feature = "wlf-lib")]
    wlf();
    #[cfg(feature = "wdb-lib")]
    wdb();
}

#[cfg(feature = "shm-lib")]
fn shm() {
    for source in [
        "native/shm.cpp",
        "native/shm.h",
        "native/shm_sst2.h",
        "native/shm_value.h",
    ] {
        println!("cargo:rerun-if-changed={source}");
    }
    assert_eq!(
        std::env::var("TARGET").unwrap(),
        "x86_64-unknown-linux-gnu",
        "shm-lib requires x86_64-unknown-linux-gnu"
    );
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("native/shm.cpp")
        .compile("ondas_shm_shim");
    println!("cargo:rustc-link-lib=dl");
}

#[cfg(feature = "wlf-lib")]
fn wlf() {
    use std::{env, fs, path::PathBuf, process::Command};
    println!("cargo:rerun-if-env-changed=QUESTA_HOME");
    println!("cargo:rerun-if-changed=native/wlf.c");
    println!("cargo:rerun-if-changed=native/wlf.h");
    println!("cargo:rerun-if-changed=native/wlf_enum.h");
    assert_eq!(
        env::var("TARGET").unwrap(),
        "x86_64-unknown-linux-gnu",
        "wlf-lib requires x86_64-unknown-linux-gnu"
    );
    let sdk = PathBuf::from(
        env::var_os("QUESTA_HOME")
            .expect("wlf-lib requires QUESTA_HOME pointing to an installed Questa SDK"),
    )
    .canonicalize()
    .expect("wlf-lib: QUESTA_HOME is not an accessible directory");
    for header in ["wlf_api.h", "mti.h", "acc_user.h", "acc_vhdl.h"] {
        let path = sdk.join("include").join(header);
        fs::File::open(&path)
            .unwrap_or_else(|error| panic!("wlf-lib: {}: {error}", path.display()));
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let library = sdk
        .join("linux_x86_64/libwlf.so")
        .canonicalize()
        .expect("wlf-lib: missing linux_x86_64/libwlf.so in QUESTA_HOME");
    println!("cargo:rerun-if-changed={}", library.display());
    let elf = Command::new("readelf")
        .arg("-d")
        .arg(&library)
        .output()
        .expect("wlf-lib: readelf (binutils) is required to check SDK linking");
    assert!(elf.status.success(), "wlf-lib: cannot inspect SDK library");
    assert!(
        !String::from_utf8_lossy(&elf.stdout).contains("(SONAME)"),
        "wlf-lib: SDK library with SONAME is unsupported"
    );
    let library = library.to_str().expect("wlf-lib: SDK path must be UTF-8");
    assert!(
        !library.contains(['\n', '\r']),
        "wlf-lib: newline in SDK path"
    );
    cc::Build::new()
        .std("c11")
        .define("__64BIT__", None)
        .flag("-isystem")
        .flag(sdk.join("include").to_str().unwrap())
        .file("native/wlf.c")
        .compile("ondas_wlf_shim");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    // A no-SONAME SDK records its absolute path in downstream DT_NEEDED entries.
    fs::write(
        out.join("libondas_wlf_sdk.so"),
        format!(
            "INPUT (\"{}\")\n",
            library.replace('\\', "\\\\").replace('"', "\\\"")
        ),
    )
    .expect("wlf-lib: write SDK linker script");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=dylib=ondas_wlf_sdk");
}

#[cfg(feature = "fsdb-lib")]
fn fsdb() {
    use std::{env, fs, path::PathBuf, process::Command};

    println!("cargo:rerun-if-env-changed=VERDI_HOME");
    println!("cargo:rerun-if-changed=native/fsdb.cpp");
    println!("cargo:rerun-if-changed=native/fsdb.h");
    println!("cargo:rerun-if-changed=native/fsdb_deps.S");
    assert_eq!(
        env::var("TARGET").unwrap(),
        "x86_64-unknown-linux-gnu",
        "fsdb-lib requires x86_64-unknown-linux-gnu"
    );
    let home = PathBuf::from(
        env::var_os("VERDI_HOME")
            .expect("fsdb-lib requires VERDI_HOME pointing to an installed Verdi FSDB Reader SDK"),
    )
    .canonicalize()
    .expect("fsdb-lib: VERDI_HOME is not an accessible directory");
    let sdk = home.join("share/FsdbReader");
    for header in ["ffrAPI.h", "ffrKit.h", "fsdbShr.h"] {
        let path = sdk.join(header);
        fs::File::open(&path).unwrap_or_else(|e| panic!("fsdb-lib: {}: {e}", path.display()));
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let lib = ["linux64", "LINUX64"]
        .into_iter()
        .map(|name| sdk.join(name))
        .find(|path| path.join("libnffr.so").is_file() && path.join("libnsys.so").is_file())
        .expect("fsdb-lib: missing libnffr.so/libnsys.so in VERDI_HOME/share/FsdbReader/linux64");
    let mut inputs = Vec::new();
    for name in ["libnffr.so", "libnsys.so"] {
        let path = lib.join(name).canonicalize().unwrap();
        println!("cargo:rerun-if-changed={}", path.display());
        let elf = Command::new("readelf")
            .arg("-d")
            .arg(&path)
            .output()
            .expect("fsdb-lib: readelf (binutils) is required to check SDK linking");
        assert!(
            elf.status.success(),
            "fsdb-lib: cannot inspect {}",
            path.display()
        );
        assert!(
            !String::from_utf8_lossy(&elf.stdout).contains("(SONAME)"),
            "fsdb-lib: SDK library with SONAME is unsupported: {}",
            path.display()
        );
        let path = path.to_str().expect("fsdb-lib: SDK path must be UTF-8");
        assert!(
            !path.contains(['\n', '\r']),
            "fsdb-lib: newline in SDK path"
        );
        inputs.push(format!(
            "\"{}\"",
            path.replace('\\', "\\\\").replace('"', "\\\"")
        ));
    }
    cc::Build::new()
        .cpp(true)
        .std("c++11")
        .flag("-isystem")
        .flag(sdk.to_str().expect("fsdb-lib: SDK path must be UTF-8"))
        .file("native/fsdb.cpp")
        .file("native/fsdb_deps.S")
        .compile("ondas_fsdb_shim");
    // rlib link-args do not propagate an RPATH to consumers. These SDK libraries
    // have no SONAME, so a native linker script retains absolute DT_NEEDED paths.
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(
        out.join("libondas_fsdb_sdk.so"),
        format!("INPUT ({})\n", inputs.join(" ")),
    )
    .expect("fsdb-lib: write local SDK linker script");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=dylib=ondas_fsdb_sdk");
    println!("cargo:rustc-link-lib=z");
}

#[cfg(feature = "wdb-lib")]
fn wdb() {
    for source in ["native/wdb.cpp", "native/wdb.h", "native/wdb_abi.hpp"] {
        println!("cargo:rerun-if-changed={source}");
    }
    assert_eq!(
        std::env::var("TARGET").unwrap(),
        "x86_64-unknown-linux-gnu",
        "wdb-lib requires x86_64-unknown-linux-gnu"
    );
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("native/wdb.cpp")
        .compile("ondas_wdb_shim");
    println!("cargo:rustc-link-lib=dl");
}
