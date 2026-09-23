pub fn greet() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greet_returns_greeting() {
        assert_eq!(greet(), "Hello, world!");
    }
}
