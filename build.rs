fn main() {
    cc::Build::new().
    file("src/c_processes.c").
    flag("-o3").
    flag("-Wall").
    compile("libc_processes.a");
}