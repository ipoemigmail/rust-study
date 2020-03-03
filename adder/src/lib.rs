#[derive(Debug)]
pub struct Rectangle {
    length: u32,
    width: u32,
}

impl Rectangle {
    fn can_hold(&self, other: &Rectangle) -> bool {
        //self.length < other.length && self.width > other.width
        self.length > other.length && self.width > other.width
    }
}

pub fn add_two(a: i32) -> i32 {
    //a + 3
    a + 2
}

fn greeting(name: &str) -> String {
    String::from("안녕하세요")
}

pub struct Guess {
    value: u32,
}

impl Guess {
    fn new(value: u32) -> Guess {
        if value < 1 {
            panic!(
                "반드시 1보다 크거나 같은 값을 사용해야 합니다. 지정된 값: {}",
                value
            );
        } else if value > 100 {
            panic!(
                "반드시 100보다 작거나 같은 값을 사용해야 합니다. 지정된 값: {}",
                value
            );
        }

        Guess { value }
    }
}

fn prints_and_returns_10(a: i32) -> i32 {
    println!("입력값: {}", a);
    10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exploration() {
        assert_eq!(2 + 2, 4);
    }

    //#[test]
    //fn another() {
    //    panic!("테스트 실패!");
    //}

    #[test]
    fn larger_can_hold_smaller() {
        let larger = Rectangle {
            length: 8,
            width: 7,
        };
        let smaller = Rectangle {
            length: 5,
            width: 1,
        };

        assert!(larger.can_hold(&smaller));
    }

    #[test]
    fn smaller_cannot_hold_larger() {
        let larger = Rectangle {
            length: 8,
            width: 7,
        };
        let smaller = Rectangle {
            length: 5,
            width: 1,
        };

        assert!(!smaller.can_hold(&larger));
    }

    #[test]
    fn it_adds_two() {
        assert_eq!(4, add_two(2));
    }

    #[test]
    fn greeting_contains_name() {
        let result = greeting("캐롤");
        assert!(
            result.contains("캐롤"),
            "Greeting 함수의 결과에 이름이 포함되어 있지 않음. 결과값: '{}'",
            result
        );
    }

    #[test]
    #[should_panic(expected = "반드시 100보다 작거나 같은 값을 사용해야 합니다.")]
    fn greater_than_100() {
        Guess::new(200);
    }

    #[test]
    fn this_test_will_pass() {
        let value = prints_and_returns_10(4);
        assert_eq!(10, value);
    }

    #[test]
    fn this_test_will_fail() {
        let value = prints_and_returns_10(8);
        assert_eq!(5, value);
    }

    #[test]
    #[ignore]
    fn expensive_test() {}
}
