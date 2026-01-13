mod app;
mod boot;
mod devices;
mod guest;
mod kvm;
mod vmm;

use clap::Parser;
use vmm::vmmon::Vmmon;

fn main() -> vmm::Result<()> {
    let args = app::cli::CliArgs::parse();
    let cfg = app::config::VmConfig::from_cli(args)?;

    let mut vm = Vmmon::build(cfg)?;
    let exit = vm.run()?;
    println!("{exit:?}");
    Ok(())
}
