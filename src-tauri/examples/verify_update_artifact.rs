fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        eprintln!("archive version channel architecture required");
        std::process::exit(2);
    }
    if flowguard_lib::verify_update_artifact_for_arch(
        std::path::Path::new(&args[0]),
        &args[1],
        &args[2],
        &args[3],
    )
    .is_err()
    {
        eprintln!("Artifact verification failed");
        std::process::exit(1);
    }
    println!("Signature, signed version, archive identity and architecture verified");
}
