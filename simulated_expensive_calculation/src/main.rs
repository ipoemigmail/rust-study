use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use std::hash::Hash;

fn main() {
    let simulated_user_specified_value = 10;
    let simulated_random_number = 7;

    generate_workout(
        simulated_user_specified_value,
        simulated_random_number
    );
}

struct Cacher<T, A, B> where T: Fn(A) -> B {
    calculation: T,
    cache: HashMap<A, B>,
}

impl<T, A: Hash + Eq + Copy, B: Copy> Cacher<T, A, B> where T: Fn(A) -> B {
    fn new(calculation: T) -> Cacher<T, A, B> {
        Cacher {
            calculation,
            cache: HashMap::new()
        }
    }

    fn value(&mut self, arg: A) -> B {
        match self.cache.get(&arg) {
            Some(v) => *v,
            None => {
                let v = (self.calculation)(arg);
                self.cache.insert(arg, v);
                v
            }
        }
    }
}

fn generate_workout(intensity: u32, random_number: u32) {
    let mut expensive_result = Cacher::new(|num: u32| {
        println!("시간이 오래 걸리는 계산을 수행 중...");
        thread::sleep(Duration::from_secs(2));
        num
    });
    if intensity < 25 {
        println!(
            "오늘은 {}번의 팔굽혀펴기를 하세요!",
            expensive_result.value(intensity)
        );
        println!(
            "다음에는 {}번의 윗몸 일으키기를 하세요!",
            expensive_result.value(intensity)
        )
    } else {
        if random_number == 3 {
            println!("오늘은 수분을 충분히 섭취하며 쉬세요!");
        } else {
            println!(
                "오늘은 {}분간 달리기를 하세요!",
                expensive_result.value(intensity)
            );
        }
    }
}

#[test]
fn call_with_different_values() {
    let mut c = Cacher::new(|a| a);

    let _v1 = c.value(1);
    let v2 = c.value(2);

    assert_eq!(v2, 2);
}
