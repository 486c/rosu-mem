pub enum Address {
    Native(usize),
    I64(i64),
    I32(i32)
}

fn test_convert<T: TryInto<usize>>(value: T) {

    let end_value: usize = value.try_into().unwrap();

    println!("")
}

fn test_xd() {
    let value_i32 = 1337i32;
    let value_i64 = 1338i64;
    let value_usize = 1339usize;


    test_convert(value_i64);

}
