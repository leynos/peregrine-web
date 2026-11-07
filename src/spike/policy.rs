//! The typed resource policy owned by the selected resource.
//!
//! Policy is data on the resource, not a route table. The specimen validates
//! scope metadata at construction so an empty requirement cannot silently mean
//! "public" or "authenticated".

/// A non-empty authorization scope name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScopeName(String);

impl ScopeName {
    /// Creates a scope name, rejecting empty and whitespace-only values.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::EmptyScopeName`] when the trimmed name is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::{PolicyError, ScopeName};
    ///
    /// let read = ScopeName::new("users:read").expect("a non-empty scope is valid");
    /// assert_eq!(read.as_str(), "users:read");
    /// assert_eq!(
    ///     ScopeName::new("  ").unwrap_err(),
    ///     PolicyError::EmptyScopeName
    /// );
    /// ```
    pub fn new(value: impl Into<String>) -> Result<Self, PolicyError> {
        let name = value.into();
        if name.trim().is_empty() {
            return Err(PolicyError::EmptyScopeName);
        }
        Ok(Self(name))
    }

    /// Returns the scope name text.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::ScopeName;
    ///
    /// let scope = ScopeName::new("invoices:write").expect("a non-empty scope is valid");
    /// assert_eq!(scope.as_str(), "invoices:write");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str { &self.0 }
}

impl AsRef<str> for ScopeName {
    fn as_ref(&self) -> &str { &self.0 }
}

/// A non-empty set of required authorization scopes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeSet(Vec<ScopeName>);

impl ScopeSet {
    /// Creates a required scope set, rejecting an empty collection.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::EmptyScopeSet`] when no scopes are supplied.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::{PolicyError, ScopeName, ScopeSet};
    ///
    /// let scope = ScopeName::new("users:read").expect("a non-empty scope is valid");
    /// let set = ScopeSet::new([scope]).expect("one scope is sufficient");
    /// assert_eq!(set.as_slice().len(), 1);
    /// assert_eq!(ScopeSet::new([]).unwrap_err(), PolicyError::EmptyScopeSet);
    /// ```
    pub fn new(scopes: impl IntoIterator<Item = ScopeName>) -> Result<Self, PolicyError> {
        let values: Vec<ScopeName> = scopes.into_iter().collect();
        if values.is_empty() {
            return Err(PolicyError::EmptyScopeSet);
        }
        Ok(Self(values))
    }

    /// Returns the scopes in declaration order.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::{ScopeName, ScopeSet};
    ///
    /// let scope = ScopeName::new("users:read").expect("a non-empty scope is valid");
    /// let set = ScopeSet::new([scope]).expect("one scope is sufficient");
    /// let names: Vec<&str> = set.as_slice().iter().map(ScopeName::as_str).collect();
    /// assert_eq!(names, ["users:read"]);
    /// ```
    #[must_use]
    pub fn as_slice(&self) -> &[ScopeName] { &self.0 }
}

/// A validation failure while constructing policy metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PolicyError {
    /// A scope name was empty or whitespace-only.
    #[error("a scope name must not be empty")]
    EmptyScopeName,
    /// A required scope set contained no scopes.
    #[error("a required scope set must contain at least one scope")]
    EmptyScopeSet,
}

/// An immutable authorization policy owned by a resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Policy {
    /// Any caller may invoke the resource.
    Public,
    /// The caller must present an authenticated identity.
    Authenticated,
    /// The caller must satisfy a non-empty set of required scopes.
    Scopes(ScopeSet),
}

impl Policy {
    /// Reports whether the policy needs no admission decision.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::Policy;
    ///
    /// assert!(Policy::Public.is_public());
    /// assert!(!Policy::Authenticated.is_public());
    /// ```
    #[must_use]
    pub const fn is_public(&self) -> bool { matches!(self, Self::Public) }

    /// Reports whether registration requires a configured enforcer.
    ///
    /// # Examples
    ///
    /// ```
    /// use peregrine_web::spike::Policy;
    ///
    /// assert!(!Policy::Public.requires_enforcement());
    /// assert!(Policy::Authenticated.requires_enforcement());
    /// ```
    #[must_use]
    pub const fn requires_enforcement(&self) -> bool { !self.is_public() }
}
