// ============================================================
// Noor CLI Binary - الـ CLI الخاص بنور
// ============================================================

use noor::core::cli::Cli;

fn main() -> noor::NoorResult<()> {
    let cli = Cli::new();
    
    let args: Vec<String> = std::env::args().skip(1).collect();
    
    if args.is_empty() {
        cli.print_help();
        return Ok(());
    }
    
    cli.run(&args)
}
