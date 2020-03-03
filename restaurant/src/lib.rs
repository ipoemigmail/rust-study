mod front_of_house;

//use front_of_house::hosting::add_to_waitlist;
//use crate::front_of_house::hosting::add_to_waitlist;
//use front_of_house::hosting;
pub use crate::front_of_house::hosting;

pub fn eat_at_restaurant() {
    //crate::front_of_house::hosting::add_to_waitlist();

    //front_of_house::hosting::add_to_waitlist();
    
    //add_to_waitlist();
    
    hosting::add_to_waitlist();

    let mut meal = back_of_house::Breakfast::summer("호밀빵");

    meal.toast = String::from("밀빵");
    println!("{} 토스트로주세요", meal.toast);

    let order1 = back_of_house::Appetizer::Soup;
    let order2 = back_of_house::Appetizer::Salad;
}

fn serve_order() {}

mod back_of_house {
    fn fix_incorrect_order() {
        cook_order();
        super::serve_order();
    }

    fn cook_order() {}

    pub struct Breakfast {
        pub toast: String,
        seasonal_fruit: String,
    }

    impl Breakfast {
        pub fn summer(toast: &str) -> Breakfast {
            Breakfast {
                toast: String::from(toast),
                seasonal_fruit: String::from("복숭아"),
            }
        }
    }

    pub enum Appetizer {
        Soup,
        Salad,
    }
}

//use std::fmt;
//use std::io;
use std::fmt::Result;
use std::io::Result as IoResult;

//fn function1() -> fmt::Result {
//    Ok(())
//}
//
//fn function2() -> io::Result<()> {
//    Ok(())
//}

fn function1() -> Result {
    Ok(())
}

fn function2() -> IoResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
