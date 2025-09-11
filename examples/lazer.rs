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
    let scaling_container_target_draw_size = Signature::from_str(
        "00 00 80 44 00 00 40 44 00 00 00 00 ?? ?? ?? ?? 00 00 00 00",
    )
    .unwrap();

    // Scan process for pre-initialized signatures
    // Be aware that osu! lazer uses usize for addresses, so we also
    // explicitly defining that
    // Also reading a values for lazer is bit more tidious than stable :)
    let scaling_container: usize =
        osu_process.read_signature(&scaling_container_target_draw_size)?;

    let external_link_opener =
        osu_process.read_i64(scaling_container - 0x24)?;

    let api = osu_process.read_i64(external_link_opener + 0x218)?;

    let game_base = osu_process.read_i64(api + 0x1f8)?;

    println!("Read a game base!");

    let beatmap_clock_ptr = osu_process.read_i64(game_base + 0x4d0)? as usize;
    let final_clock_ptr =
        osu_process.read_i64(beatmap_clock_ptr + 0x210)? as usize;

    let current_time = osu_process.read_f64(final_clock_ptr + 0x30)?;

    println!("Current osu time: {current_time}");

    let storage = osu_process.read_i64(game_base + 0x440)?;
    let underlying_storage = osu_process.read_i64(storage + 0x10)?;

    let base_path =
        osu_process.read_string_from_ptr(underlying_storage + 0x08)?;

    println!("Base path: {base_path}");

    Ok(())
}
