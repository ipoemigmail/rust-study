use num_bigint::BigUint;
use std::env;

fn main() {
    let args: Vec<_> = env::args().collect();
    let n = args[1].parse::<u64>().unwrap();
    let bi = BigUint::from(n);
    let r = factorial(bi);
    println!("{}", r);
}

fn factorial(n: BigUint) -> BigUint {
    #[tco::rewrite]
    fn aux(c: BigUint, r: BigUint) -> BigUint {
        match c {
            _ if c < BigUint::from(2_u32) => r,
            _ => aux(c.clone() - BigUint::from(1_u32), r * c),
        }
    }
    aux(n, BigUint::from(1_u32))
}
