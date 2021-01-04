pub trait HelloService {
    fn hello(&self) -> String;
}

pub struct HelloServiecDefault;

impl HelloService for HelloServiecDefault {
    fn hello(&self) -> String {
        "hello".to_string()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn run_test() {
        println!()
    }
}
