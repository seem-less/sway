script;

use std::types::B512;

fn main() -> bool {
    let hi_bits: b256 = 0x7777777777777777777777777777777777777777777777777777777777777777;
    let lo_bits: b256 = 0x5555555555555555555555555555555555555555555555555555555555555555;

    let b: B512 = ~B512::from_b_256(hi_bits, lo_bits);

    b.lo == lo_bits && b.hi == hi_bits
}
