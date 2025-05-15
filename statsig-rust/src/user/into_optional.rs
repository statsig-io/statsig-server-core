pub trait IntoOptional<T> {
    fn into_optional(self) -> Option<T>;
}

impl<T> IntoOptional<T> for T {
    fn into_optional(self) -> Option<T> {
        Some(self)
    }
}

impl<T: Clone> IntoOptional<T> for &T {
    fn into_optional(self) -> Option<T> {
        Some(self.clone())
    }
}

impl<T> IntoOptional<T> for Option<T> {
    fn into_optional(self) -> Option<T> {
        self
    }
}

impl<T: Clone> IntoOptional<T> for Option<&T> {
    fn into_optional(self) -> Option<T> {
        self.cloned()
    }
}

impl IntoOptional<String> for &str {
    fn into_optional(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoOptional<String> for Option<&str> {
    fn into_optional(self) -> Option<String> {
        self.map(|s| s.to_string())
    }
}
