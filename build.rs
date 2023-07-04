fn main() {
    cxx_build::bridge("src/lib.rs")
        .file("csrc/ffi.cpp")
        .flag_if_supported("-std=c++17")
        .compile("egg-netlist-synthesizer");
}
