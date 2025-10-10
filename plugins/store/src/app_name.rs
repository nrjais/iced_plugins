//! Application name configuration for storage paths

/// Application identifier used to determine storage location
///
/// The store uses the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html)
/// on Linux and similar conventions on other platforms.
///
/// # Example
///
/// ```
/// use iced_store_plugin::AppName;
///
/// let app_name = AppName::new("com", "example", "myapp");
/// ```
#[derive(Clone, Debug)]
pub struct AppName {
    pub qualifier: String,
    pub organization: String,
    pub application: String,
}

impl AppName {
    /// Create a new application name
    ///
    /// # Arguments
    ///
    /// * `qualifier` - Typically a reverse domain name (e.g., "com", "org")
    /// * `organization` - Your organization or username (e.g., "mycompany")
    /// * `application` - The application name (e.g., "myapp")
    ///
    /// # Example
    ///
    /// ```
    /// use iced_store_plugin::AppName;
    ///
    /// let app_name = AppName::new("com", "acme", "roadrunner");
    /// ```
    pub fn new(
        qualifier: impl Into<String>,
        organization: impl Into<String>,
        application: impl Into<String>,
    ) -> Self {
        Self {
            qualifier: qualifier.into(),
            organization: organization.into(),
            application: application.into(),
        }
    }
}
