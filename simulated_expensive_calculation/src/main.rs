use List::{Cons, Nil};
use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    //let simulated_user_specified_value = 10;
    //let simulated_random_number = 7;

    //generate_workout(
    //    simulated_user_specified_value,
    //    simulated_random_number
    //);

    let a = Rc::new(Cons(5, Rc::new(Cons(10, Rc::new(Nil)))));
    println!("a를 생성한 후의 카운터 = {}", Rc::strong_count(&a));
    let b = Cons(3, Rc::clone(&a));
    println!("b를 생성한 후의 카운터 = {}", Rc::strong_count(&a));
    {
        let c = Cons(4, Rc::clone(&a));
        println!("c를 생성한 후의 카운터 = {}", Rc::strong_count(&a));
    }
    println!("c가 범위를 벗어난 후의 카운터 = {}", Rc::strong_count(&a));
}

enum List {
    Cons(i32, Rc<List>),
    Nil,
}

trait Messenger {
    fn send(&self, msg: &str);
}

struct LimitTracker<'a, T: 'a + Messenger> {
    messenger: &'a T,
    value: usize,
    max: usize,
}

impl<'a, T> LimitTracker<'a, T> where T: Messenger {
    pub fn new(messenger: &T, max: usize) -> LimitTracker<T> {
        LimitTracker {
            messenger,
            value: 0,
            max,
        }
    }

    pub fn set_value(&mut self, value: usize) {
        self.value = value;

        let percentage_of_max = self.value as f64 / self.max as f64;

        if percentage_of_max >= 0.75 && percentage_of_max < 0.9 {
            self.messenger.send("경고");
        } else if percentage_of_max >= 0.9 && percentage_of_max < 1.0 {
            self.messenger.send("긴급 경고");
        } else if percentage_of_max >= 1.0 {
            self.messenger.send("에러");
        }
    }
}

struct MockMessenger {
    sent_messages: RefCell<Vec<String>>,
}

impl MockMessenger {
    fn new() -> MockMessenger {
        MockMessenger {
            sent_messages: RefCell::new(vec![])
        }
    }

    fn new_box() -> MockMessenger {
        *Box::new(MockMessenger {
            sent_messages: RefCell::new(vec![])
        })
    }
}

impl Messenger for MockMessenger {
    fn send(&self, msg: &str) {
        self.sent_messages.borrow_mut().push(String::from(msg));
    }
}


//struct Cacher<T, A, B> where T: Fn(A) -> B {
//    calculation: T,
//    cache: HashMap<A, B>,
//}
//
//impl<T, A: Hash + Eq + Copy, B: Copy> Cacher<T, A, B> where T: Fn(A) -> B {
//    fn new(calculation: T) -> Cacher<T, A, B> {
//        Cacher {
//            calculation,
//            cache: HashMap::new()
//        }
//    }
//
//    fn value(&mut self, arg: A) -> B {
//        match self.cache.get(&arg) {
//            Some(v) => *v,
//            None => {
//                let v = (self.calculation)(arg);
//                self.cache.insert(arg, v);
//                v
//            }
//        }
//    }
//}
//
//fn generate_workout(intensity: u32, random_number: u32) {
//    let mut expensive_result = Cacher::new(|num: u32| {
//        println!("시간이 오래 걸리는 계산을 수행 중...");
//        thread::sleep(Duration::from_secs(2));
//        num
//    });
//    if intensity < 25 {
//        println!(
//            "오늘은 {}번의 팔굽혀펴기를 하세요!",
//            expensive_result.value(intensity)
//        );
//        println!(
//            "다음에는 {}번의 윗몸 일으키기를 하세요!",
//            expensive_result.value(intensity)
//        )
//    } else {
//        if random_number == 3 {
//            println!("오늘은 수분을 충분히 섭취하며 쉬세요!");
//        } else {
//            println!(
//                "오늘은 {}분간 달리기를 하세요!",
//                expensive_result.value(intensity)
//            );
//        }
//    }
//}
//
//#[test]
//fn call_with_different_values() {
//    let mut c = Cacher::new(|a| a);
//
//    let _v1 = c.value(1);
//    let v2 = c.value(2);
//
//    assert_eq!(v2, 2);
//}
