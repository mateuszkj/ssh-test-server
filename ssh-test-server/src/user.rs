/// Ssh user
#[derive(Debug)]
pub struct User {
    login: String,
    password: String,
    admin: bool,
}

impl User {
    pub fn new(login: &str, password: &str) -> Self {
        Self {
            login: login.to_string(),
            password: password.to_string(),
            admin: false,
        }
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
}
