/// Ssh user
#[derive(Clone, Debug)]
pub struct User {
    login: String,
    password: String,
    admin: bool,
}

impl User {
    pub fn new<L: Into<String>, P: Into<String>>(login: L, password: P) -> Self {
        Self {
            login: login.into(),
            password: password.into(),
            admin: false,
        }
    }

    pub fn new_admin<L: Into<String>, P: Into<String>>(login: L, password: P) -> Self {
        let mut u = Self::new(login, password);
        u.set_admin(true);
        u
    }

    pub fn set_admin(&mut self, admin: bool) {
        self.admin = admin;
    }

    pub fn admin(&self) -> bool {
        self.admin
    }

    pub fn login(&self) -> &str {
        &self.login
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn set_password(&mut self, new_password: &str) {
        self.password = new_password.to_string();
    }
}
