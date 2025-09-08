use paste::paste;
use rand::{distributions::Alphanumeric, prelude::*};
use std::io::Read;

use rosu_mem::{error::*, process::ProcessTraits};

macro_rules! prim_read_test {
    ($t: ident) => {
        paste! {
            #[test]
            fn [<test_ $t>]() {
                let mut rng = rand::thread_rng();
                let num: $t = rng.gen();

                let buff: Vec<u8> = num.to_le_bytes().to_vec();
                let p = FakeProccess {
                    buff
                };

                let tmp = p.[<read_ $t>](0).unwrap();
                assert_eq!(tmp, num);
            }

        }
    };
}

macro_rules! prim_read_array_test {
    ($t: ident) => {
        paste! {
            #[test]
            fn [<test_ $t _array>]() {
                // Constructing fake array identical how arrays
                // stored inside osu!
                // Type for this example is i32
                // | 00 00 00 00 | 0C 00 00 00 | 00 00 00 00 |
                // ^ [1]         ^ [2]         ^ [1]
                // | 00 00 00 01 | 00 00 00 00 |
                // ^ [3]         ^ [4]
                //
                // [1] - Bytes we don't care about
                // [2] - Pointer to our data, since we
                // are inside test then it's
                // just a index to our test array
                // [3] - Size of array
                // [4] - Array of our desired type
                // Result of this example gonna be -> [0; 1]
                let mut rng = rand::thread_rng();

                let mut synthetic_buff = Vec::new();
                let items_start_idx: i32 = 8;
                let length: i32 = rng.gen_range(0..1024);
                let items: Vec<$t> = (0..length)
                    .map(|_| {
                        rand::random::<$t>()
                    }).collect();

                synthetic_buff.extend(
                    [0u8; 4]
                ); // Bytes we don't care about
                synthetic_buff.extend(
                    items_start_idx.to_le_bytes()
                ); // Index to array data
                synthetic_buff.extend(
                    [0u8; 4]
                ); // Bytes we don't care about
                synthetic_buff.extend(
                    length.to_le_bytes()
                ); // Size of array

                // Filling our synthetic array with random generated
                // numbers
                for item in &items {
                    synthetic_buff.extend(item.to_le_bytes())
                }

                let p = FakeProccess {
                    buff: synthetic_buff
                };

                let mut fake_output_buff = Vec::new();

                p.[<read_ $t _array>](0, &mut fake_output_buff)
                    .unwrap();

                assert_eq!(fake_output_buff.len(), length as usize);
                assert_eq!(fake_output_buff, items);
            }
        }
    };
}

pub struct FakeProccess {
    buff: Vec<u8>,
}

impl ProcessTraits for FakeProccess {
    fn initialize(
        _proc_name: &str,
        _exclude: &[&str],
    ) -> Result<Self, ProcessError> {
        todo!()
    }

    fn find_process(
        _proc_name: &str,
        _exclude: &[&str],
    ) -> Result<Self, ProcessError> {
        todo!()
    }

    fn read_regions(self) -> Result<Self, ProcessError> {
        todo!()
    }

    fn read_signature<T: TryFrom<usize>>(
        &self,
        _sign: &rosu_mem::signature::Signature,
    ) -> Result<T, ProcessError> {
        todo!()
    }

    fn read<T: TryInto<usize>>(
        &self,
        addr: T,
        len: usize,
        buff: &mut [u8],
    ) -> Result<(), ProcessError> {
        let addr: usize = addr
            .try_into()
            .map_err(|_| ProcessError::AddressConvertError)?;
        // Addr - starting index
        // self.buff.set_position(addr as u64);
        // self.buff.read(buff);

        let mut slice = &self.buff[addr..addr + len];
        let _ = slice.read(buff);

        Ok(())
    }
}

#[test]
fn test_uleb() {
    let buff = vec![
        0x0B, 0x04, 0x74, 0x65, 0x73, 0x74, 0x00, 0x00, 0x00, 0x00, 0x0A,
    ];

    let p = FakeProccess { buff };

    // Skipping that one 0x0b byte
    assert_eq!(p.read_uleb128(1).unwrap(), 4);
}

#[test]
fn test_string() {
    let mut rng = thread_rng();

    for len in [0u32, 1, 2, 4, 8, 16, 32] {
        let mut buff = vec![0x0; 4]; // Random 4 bytes
        buff.extend_from_slice(&len.to_le_bytes());

        let random_string: String =
            (0..len).map(|_| rng.sample(Alphanumeric) as char).collect();

        // convert to UTF-16 bytes
        let random_string_bytes =
            random_string.bytes().flat_map(|byte| [byte, 0]);

        buff.extend(random_string_bytes);

        let p = FakeProccess { buff };

        let read_string = p.read_string(0).unwrap();
        assert_eq!(read_string, random_string);
    }
}

#[test]
fn test_string_with_limit_fail_all() {
    let mut rng = thread_rng();

    for len in [10u32, 16, 32, 100, 256, 1024] {
        let mut buff = vec![0x0; 4]; // Random 4 bytes
        buff.extend_from_slice(&len.to_le_bytes());

        let random_string: String =
            (0..len).map(|_| rng.sample(Alphanumeric) as char).collect();

        // convert to UTF-16 bytes
        let random_string_bytes =
            random_string.bytes().flat_map(|byte| [byte, 0]);

        buff.extend(random_string_bytes);

        let p = FakeProccess { buff };

        let read_string = p.read_string_with_limit(0, 8);
        assert!(read_string.is_err());
        assert!(matches!(read_string, Err(ProcessError::StringTooLarge)))
    }
}

#[test]
fn test_string_with_limit_no_fail() {
    let mut rng = thread_rng();

    for len in [10u32, 16, 32, 100, 256, 1024, 2047] {
        let mut buff = vec![0x0; 4]; // Random 4 bytes
        buff.extend_from_slice(&len.to_le_bytes());

        let random_string: String =
            (0..len).map(|_| rng.sample(Alphanumeric) as char).collect();

        // convert to UTF-16 bytes
        let random_string_bytes =
            random_string.bytes().flat_map(|byte| [byte, 0]);

        buff.extend(random_string_bytes);

        let p = FakeProccess { buff };

        let read_string = p.read_string_with_limit(0, 2048);
        assert!(!read_string.is_err());
        let read_string = read_string.unwrap();
        assert_eq!(read_string, random_string);
    }
}

#[test]
fn test_string_from_ptr_simple() {
    let mut rng = thread_rng();

    let string_len: u32 = 64;
    let pointer_points_at: i32 = 4; // 4 bytes ahead

    let mut buff: Vec<u8> = Vec::new();
    buff.extend_from_slice(&pointer_points_at.to_le_bytes());

    assert_eq!(buff.len(), 4);

    buff.extend_from_slice(&[0x0, 0x0, 0x0, 0x0]); // Random 4 bytes
    buff.extend_from_slice(&string_len.to_le_bytes()); // 4B string length

    assert_eq!(buff.len(), 12);

    let random_string: String = (0..string_len)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect();

    // convert to UTF-16 bytes
    let random_string_bytes = random_string.bytes().flat_map(|byte| [byte, 0]);

    buff.extend(random_string_bytes);

    let p = FakeProccess { buff };

    let read_string = p.read_string_from_ptr(0).unwrap();
    assert_eq!(read_string, random_string);
}

#[test]
fn test_string_from_ptr_randomized() {
    let mut rng = thread_rng();

    for _ in 0..=1337 {
        let string_len: u32 = rng.gen_range(1..200);
        let pointer_points_at: i32 = rng.gen_range(5..1000); // Starts at five to account for the sizeof(pointer)

        let mut buff = Vec::with_capacity(pointer_points_at as usize);

        buff.extend_from_slice(&pointer_points_at.to_le_bytes());
        assert_eq!(buff.len(), 4);

        let to_fill = pointer_points_at - 4;
        buff.extend_from_slice(&vec![0u8; to_fill as usize]);

        assert_eq!(buff.len(), pointer_points_at as usize);

        buff.extend_from_slice(&[0x0, 0x0, 0x0, 0x0]); // Random 4 bytes
        buff.extend_from_slice(&string_len.to_le_bytes()); // 4B string length

        let random_string: String = (0..string_len)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();

        // convert to UTF-16 bytes
        let random_string_bytes =
            random_string.bytes().flat_map(|byte| [byte, 0]);

        buff.extend(random_string_bytes);

        // Checking if buffer is pointer_points_at + 4B obj header + 4B length + len(UTF16 string itself)
        assert_eq!(
            buff.len(),
            (pointer_points_at + 4 + 4 + (string_len as i32 * 2)) as usize
        );

        let p = FakeProccess { buff };

        let read_string = p.read_string_from_ptr(0).unwrap();
        assert_eq!(read_string, random_string);
    }
}

prim_read_test!(i8);
prim_read_test!(i16);
prim_read_test!(i32);
prim_read_test!(i64);
prim_read_test!(i128);

prim_read_test!(u8);
prim_read_test!(u16);
prim_read_test!(u32);
prim_read_test!(u64);
prim_read_test!(u128);

prim_read_test!(f32);
prim_read_test!(f64);

prim_read_array_test!(i8);
prim_read_array_test!(i16);
prim_read_array_test!(i32);
prim_read_array_test!(i64);
prim_read_array_test!(i128);

prim_read_array_test!(u8);
prim_read_array_test!(u16);
prim_read_array_test!(u32);
prim_read_array_test!(u64);
prim_read_array_test!(u128);

prim_read_array_test!(f32);
prim_read_array_test!(f64);
