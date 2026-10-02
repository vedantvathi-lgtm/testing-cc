fn main() {
    cc::Build::new()
        .file("nsight_capture.c")
        .include(".")
        .compile("nsight_capture");
    println!("cargo:rerun-if-changed=nsight_capture.c");
    println!("cargo:rerun-if-changed=nsight_capture.h");
}
