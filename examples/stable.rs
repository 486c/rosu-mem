use rosu_mem::{
    error::ProcessError,
    process::{Process, ProcessTraits},
    signature::Signature,
};
use std::str::FromStr;

// Exclude words, basically a hack to properly find a osu! process when using wine
static EXCLUDE_WORDS: [&str; 2] = ["umu-run", "waitforexitandrun"];

fn main() -> Result<(), ProcessError> {
    // Initialize a process first
    let osu_process = Process::initialize("osu!.exe", &EXCLUDE_WORDS)?;

    println!("Found a osu! process");

    // Initialize a signatures
    let base_signature = Signature::from_str("F8 01 74 04 83 65").unwrap();
    let status_signature = Signature::from_str("48 83 F8 04 73 1E").unwrap();

    // Scan process for pre-initialized signatures
    // Be aware that osu! stable uses i32 for addresses, so we also
    // explicitly defining that

    // Reading a base signature just to be sure that we are in the correct osu! process
    let _base: i32 = osu_process.read_signature(&base_signature)?;
    let status: i32 = osu_process.read_signature(&status_signature)?;

    println!("Found all required signatures!");

    // Now read the values that you are intrested in :)
    // For the sake of keeping example simple we will read a current game state
    let status_ptr = osu_process.read_i32(status - 0x4)?;
    let osu_state_status = osu_process.read_u32(status_ptr)?;

    println!("Current osu! game status: {osu_state_status}");

    Ok(())
}
