/// Ssh user.
///
/// # Example
///
/// ```
/// use ssh_test_server::User;
/// let user = User::new("user", "password");
/// assert_eq!(user.login(), "user");
/// ```
#[derive(Clone, Debug)]
pub struct User {
    login: String,
    password: String,
    admin: bool,
}

impl User {
    /// Create new user with password.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::User;
    /// let u = User::new("ala", "kot");
    /// assert_eq!(u.login(), "ala");
    /// assert_eq!(u.password(), "kot");
    /// ```
    pub fn new<L: Into<String>, P: Into<String>>(login: L, password: P) -> Self {
        Self {
            login: login.into(),
            password: password.into(),
            admin: false,
        }
    }

    /// Create new user with admin flag.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::User;
    /// let u = User::new_admin("Administrator", "admin1");
    /// assert!(u.admin());
    /// ```
    pub fn new_admin<L: Into<String>, P: Into<String>>(login: L, password: P) -> Self {
        let mut u = Self::new(login, password);
        u.set_admin(true);
        u
    }

    /// Modify admin flag.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::User;
    /// let mut u = User::new("login", "password");
    /// assert!(!u.admin());
    ///
    /// u.set_admin(true);
    /// assert!(u.admin());
    /// ```
    pub fn set_admin(&mut self, admin: bool) {
        self.admin = admin;
    }

    /// Get admin flag.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::User;
    /// let mut u = User::new("login", "password");
    /// assert!(!u.admin());
    /// ```
    pub fn admin(&self) -> bool {
        self.admin
    }

    /// Get user's login.
    pub fn login(&self) -> &str {
        &self.login
    }

    /// Get user's password.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::User;
    /// let u = User::new("a", "12345");
    /// assert_eq!(u.password(), "12345");
    /// ```
    pub fn password(&self) -> &str {
        &self.password
    }

    /// Modify user's password.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::User;
    /// let mut u = User::new("a", "12");
    /// assert_eq!(u.password(), "12");
    ///
    /// u.set_password("34");
    /// assert_eq!(u.password(), "34");
    /// ```
    pub fn set_password(&mut self, new_password: &str) {
        self.password = new_password.to_string();
    }
}
