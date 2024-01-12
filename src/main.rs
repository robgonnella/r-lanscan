use clap::Parser;

/// Local Area Network ARP and SYN scanning
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Comma separated list of IPs, IP ranges, and CIDR blocks to scan
    #[arg(short, long, default_value = "")]
    targets: String,

    /// Comma separated list of ports and port ranges to scan
    #[arg(short, long, default_value = "1-65535")]
    ports: String,

    /// Output final report in json instead of table text
    #[arg(short, long, default_value_t = false)]
    json: bool,

    /// Perform only an ARP scan (omits SYN scanning)
    #[arg(short, long, default_value_t = false)]
    arp_only: bool,

    /// Choose a specific network interface for the scan
    #[arg(short, long, default_value = "")]
    interface: String,
}

fn main() {
    let args = Args::parse();

    println!("configuration:");
    println!("  targets: {}", args.targets);
    println!("  ports: {}", args.ports);
    println!("  json: {}", args.json);
    println!("  arpOnly: {}", args.arp_only);
    println!("  interface: {}", args.interface);
}
