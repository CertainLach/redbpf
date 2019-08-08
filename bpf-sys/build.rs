use std::env;
use std::path::PathBuf;
use std::fs;

pub const KERNEL_HEADERS: [&'static str; 6] = [
    "arch/x86/include/generated/uapi",
    "arch/x86/include/uapi",
    "arch/x86/include/",
    "include/generated/uapi",
    "include/uapi",
    "include",
];

pub mod uname {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/uname.rs"));
}

pub mod headers {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/headers.rs"));
}

fn main() {
    println!("cargo:rustc-link-lib=static=bpf");

    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(out_dir);

    let mut libbpf = cc::Build::new();
    libbpf
        .flag("-Wno-sign-compare")
        .flag("-Wno-int-conversion")
        .include("libbpf/include/uapi")
        .include("libbpf/include")
        .include("bcc")
        .include(".");
    if target.contains("musl") {
        let path = PathBuf::from(headers::kernel_headers_path().expect("couldn't find kernel headers"));
        for rel_dir in &KERNEL_HEADERS {
            libbpf.include(path.join(rel_dir).to_str().unwrap());
        }
        let libelf_path = out_path.join("libelf");
        copy_libelf_headers(&libelf_path);
        libbpf
            .define("COMPAT_NEED_REALLOCARRAY", "1")
            .include(libelf_path);
    }
    libbpf
        .flag("-include").flag("linux/stddef.h")
        .file("libbpf/src/bpf.c")
        .file("libbpf/src/bpf_prog_linfo.c")
        .file("libbpf/src/btf.c")
        .file("libbpf/src/libbpf.c")
        .file("libbpf/src/libbpf_errno.c")
        .file("libbpf/src/libbpf_probes.c")
        .file("libbpf/src/netlink.c")
        .file("libbpf/src/nlattr.c")
        .file("libbpf/src/str_error.c")
        .file("libbpf/src/xsk.c")
        .file("bcc/libbpf.c")
        .file("bcc/perf_reader.c")
        .compile("libbpf.a");

    let bindings = bindgen::Builder::default()
        .header("bcc/libbpf.h")
        .generate()
        .expect("Unable to generate bindings");
    bindings
        .write_to_file(out_path.join("libbpf_bindings.rs"))
        .expect("Couldn't write bindings!");
    let bindings = bindgen::Builder::default()
        .header("bcc/perf_reader.h")
        .generate()
        .expect("Unable to generate bindings");
    bindings
        .write_to_file(out_path.join("perf_reader_bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn copy_libelf_headers(out_path: &PathBuf) {
    let libelf_prefix = "/usr/include"; // FIXME: find this with pkg-config
    let libelf_path = PathBuf::from(libelf_prefix);

    let _ = fs::create_dir(out_path);
    for header in &["libelf.h", "gelf.h", "nlist.h"] {
        let input = libelf_path.join(header);
        let output = out_path.join(header);
        fs::copy(input, output).expect(&format!("couldn't copy {}", header));
    }
}