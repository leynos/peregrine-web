//! The frozen HTTP method used by the resource-policy spike.
//!
//! The effective method is decided before admission and then carried unchanged
//! into the resource. Freezing it as a small `Copy` value keeps policy lookup
//! and responder selection from re-reading mutable request state later.

/// The HTTP method frozen before resource policy admission.
///
/// The specimen models only the standard methods. Route parsing and per-method
/// dispatch remain outside this spike.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Method {
    /// The HTTP `GET` method.
    Get,
    /// The HTTP `POST` method.
    Post,
    /// The HTTP `PUT` method.
    Put,
    /// The HTTP `DELETE` method.
    Delete,
    /// The HTTP `PATCH` method.
    Patch,
    /// The HTTP `HEAD` method.
    Head,
    /// The HTTP `OPTIONS` method.
    Options,
}

impl Method {
    /// Maps an `http` method to the spike's frozen method.
    ///
    /// Returns [`None`] for an extension method that the specimen does not
    /// model.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::Method;
    ///
    /// assert_eq!(Method::from_http(&http::Method::GET), Some(Method::Get));
    /// assert_eq!(
    ///     Method::from_http(&http::Method::DELETE),
    ///     Some(Method::Delete)
    /// );
    /// ```
    #[must_use]
    pub fn from_http(method: &http::Method) -> Option<Self> {
        match method.as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "DELETE" => Some(Self::Delete),
            "PATCH" => Some(Self::Patch),
            "HEAD" => Some(Self::Head),
            "OPTIONS" => Some(Self::Options),
            _ => None,
        }
    }

    /// Returns the canonical method spelling.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::Method;
    ///
    /// assert_eq!(Method::Get.as_str(), "GET");
    /// assert_eq!(Method::Options.as_str(), "OPTIONS");
    /// ```
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}
