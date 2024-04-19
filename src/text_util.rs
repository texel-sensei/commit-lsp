use std::borrow::Cow;

/// Truncate and ellipse strings in a human-friendly way.
pub trait Ellipse {
    type Output;

    /// Truncate to a length of `len` extended grapheme clusters and place the given
    /// ellipse string at the end when truncating.
    ///
    /// Truncating to a length of 0 will yield the empty element without an
    /// attached ellipsis.
    fn truncate_ellipse_with(&self, len: usize, ellipse: &str) -> Self::Output;

    /// Truncate to a length of `len` extended grapheme clusters and add `...` at
    /// the end of the string when truncating.
    ///
    /// Truncating to a length of 0 will yield the empty element without an
    /// attached ellipsis.
    fn truncate_ellipse(&self, len: usize) -> Self::Output {
        self.truncate_ellipse_with(len, "...")
    }
}

impl<'a> Ellipse for &'a str {
    type Output = Cow<'a, str>;

    fn truncate_ellipse_with(&self, len: usize, ellipse: &str) -> Self::Output {
        if self.chars().count() <= len {
            return Cow::Borrowed(self);
        } else if len == 0 {
            return Cow::Borrowed("");
        }

        let result = self
            .chars()
            .take(len)
            .chain(ellipse.chars())
            .collect();
        Cow::Owned(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(&"Hello, World!".truncate_ellipse(3), "Hel...",);
    }

    #[test]
    fn ellipse_with() {
        assert_eq!(&"Hello, World!".truncate_ellipse_with(3, "---"), "Hel---");
    }

    #[test]
    fn truncate_to_empty() {
        assert_eq!(&"Hello, World!".truncate_ellipse(0), "");
    }
}
