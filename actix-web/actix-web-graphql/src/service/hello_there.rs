use super::hello::HelloService;
use typed_builder::TypedBuilder;
pub trait HelloThereService {
    fn hello_there(&self, to: &str) -> String;
}

#[derive(Clone, TypedBuilder)]
pub struct HelloThereServiceDefault<A: HelloService> {
    hello_service: A,
}

impl<A: HelloService> HelloThereService for HelloThereServiceDefault<A> {
    fn hello_there(&self, to: &str) -> String {
        format!("{} {}", self.hello_service.hello(), to)
    }
}
