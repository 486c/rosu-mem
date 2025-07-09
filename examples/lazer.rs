use rosu_mem::{
    error::ProcessError,
    process::{Process, ProcessTraits},
    signature::Signature,
};
use std::str::FromStr;

fn main() -> Result<(), ProcessError> {
    // Initialize a process first
    let osu_process = Process::initialize("osu!", &[])?;

    println!("Found a osu! lazer process");

    // Initialize a signatures
    let session_signature =
        Signature::from_str("00 00 00 00 80 4F 12 41").unwrap();

    // Scan process for pre-initialized signatures
    // Be aware that osu! lazer uses usize for addresses, so we also
    // explicitly defining that
    // Also reading a values for lazer is bit more tidious than stable :)
    let mut session_addr: usize =
        osu_process.read_signature(&session_signature)?; //- 0x208;

    // Read values! For simplicity willl read only a current game time
    session_addr -= 0x208;

    let one = (osu_process.read_i64(session_addr + 0x90)? + 0x90) as usize;
    let two = (osu_process.read_i64(one)? + 0x90) as usize;
    let three = (osu_process.read_i64(two)? + 0x90) as usize;
    let four = (osu_process.read_i64(three)? + 0x90) as usize;
    let five = (osu_process.read_i64(four)? + 0x90) as usize;
    let six = (osu_process.read_i64(five)? + 0x90) as usize;
    let seven = (osu_process.read_i64(six)? + 0x340) as usize;

    let game_base = osu_process.read_i64(seven)? as usize;

    println!("Read a game base!");

    let beatmap_clock_ptr = osu_process.read_i64(game_base + 0x4d0)? as usize;
    let final_clock_ptr =
        osu_process.read_i64(beatmap_clock_ptr + 0x210)? as usize;

    let current_time = osu_process.read_f64(final_clock_ptr + 0x30)?;

    println!("Current osu time: {current_time}");

    Ok(())
}
