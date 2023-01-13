pub fn hold_a_name(name: String) -> String {
    format!("Holding a name `{}`", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = hold_a_name(String::from("panacea"));
        assert_eq!(result, String::from("Holding a name `panacea`"));
    }
}
