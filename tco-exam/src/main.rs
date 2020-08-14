fn main() {
    let r = factorial(20);
    println!("{}", r);
}

fn factorial(n: u64) -> u64 {
    #[tco::rewrite]
    fn aux(c: u64, r: u64) -> u64 {
        match c {
            _ if c < 2 => r,
            _ => aux(c - 1, r * c),
        }
    }
    aux(n, 1)
}
