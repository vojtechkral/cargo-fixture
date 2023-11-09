pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);

        let foo = env::var("FOO").unwrap();
        assert_eq!(foo, "bar");
    }

    #[test]
    #[ignore]
    fn it_works_2() {
        let result = add(2, 2);
        assert_eq!(result, 4);

        let foo = env::var("FOO").unwrap();
        assert_eq!(foo, "bar");

        panic!("Ha!");
    }
}
