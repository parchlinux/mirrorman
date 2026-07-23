use std::env;

use mirrorman::mirror_manager;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--help".to_string())
        || args.contains(&"-h".to_string())
        || args.len() <= 1
    {
        println!("mirrorman-cli - Command Line Interface for Parch Mirror Manager\n");
        println!("Usage: mirrorman-cli [OPTIONS]\n");
        println!("Options:");
        println!("  --refresh     Fetch latest mirror status from Arch API");
        println!("  --best-setup  Apply best setup (select top reliable mirrors across countries)");
        println!("  --test        Test mirrors speed concurrently");
        println!("  --save        Save mirrorlist to /etc/pacman.d/mirrorlist via D-Bus service");
        println!("  -h, --help    Show this help message");
        return;
    }

    let mut mgr = mirror_manager::MirrorManager::new();

    if args.contains(&"--refresh".to_string())
        || args.contains(&"--best-setup".to_string())
        || args.contains(&"--test".to_string())
    {
        println!("[+] Fetching mirrors status...");
        if let Err(e) = mgr.fetch_mirrors(None, &[], &["4".to_string(), "6".to_string()], true) {
            eprintln!("[!] Error fetching mirrors: {e}");
            std::process::exit(1);
        }
        println!("[+] Fetched {} mirrors.", mgr.mirrors.len());
    }

    if args.contains(&"--test".to_string()) {
        println!("[+] Testing mirror response times...");
        mirror_manager::MirrorManager::test_all_speeds_concurrent(&mut mgr.mirrors, 50);
        mgr.sort_by_speed();
    }

    if args.contains(&"--best-setup".to_string()) {
        println!("[+] Running auto-optimization (Best Setup)...");
        let selected = mgr.auto_optimize();
        println!("[+] Selected {} optimal mirrors:", selected.len());
        for m in &selected {
            println!("    - {} ({})", m.url, m.country);
        }
    }

    if args.contains(&"--save".to_string()) {
        println!("[+] Saving mirrorlist to /etc/pacman.d/mirrorlist...");
        if let Err(e) = mgr.save_mirrorlist() {
            eprintln!("[!] Failed to save mirrorlist: {e}");
            std::process::exit(1);
        }
        println!("[+] Mirrorlist saved successfully!");
    }
}
