#![allow(dead_code)]

use asession::{Session, SessionBuilder};
use chrono::Local;
use std::ops::Deref;

// print only when debug mode is on
macro_rules! dp {
    ($e:expr) => {
        if cfg!(debug_assertions) {
            dbg!($e);
        }
    };
}

pub struct Client {
    session: Session,
    status: Option<String>,
}

impl Client {
    pub fn new() -> Self {
        let client = SessionBuilder::new()
            .cookies_store_into("cookies".into())
            .build()
            .unwrap();

        Self {
            session: client,
            status: None,
        }
    }

    pub async fn get_ticket(
        &self,
        uid: &str,
        password: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let res = self
            .session
            .post("https://hkuportal.hku.hk/cas/servlet/edu.yale.its.tp.cas.servlet.Login")
            .form(&[("keyid", ""), ("username", uid), ("password", password)])
            .send()
            .await
            .unwrap();

        // get ticket and url
        let body = res.text().await.unwrap();
        let url = regex::Regex::new(r#"Click <a href="(.*)">here</a>"#)
            .unwrap()
            .captures(&body)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();

        Ok(url.split("=").last().unwrap().to_string())
    }

    pub async fn login_portal(
        &self,
        uid: &str,
        password: &str,
    ) -> Result<&Self, Box<dyn std::error::Error>> {
        dp!("-start login to portal");
        dp!("--passing uid and password");
        let res = self
            .session
            .post("https://hkuportal.hku.hk/cas/servlet/edu.yale.its.tp.cas.servlet.Login")
            .form(&[("keyid", ""), ("username", uid), ("password", password)])
            .send()
            .await?;

        dp!("--get ticket and url");
        let body = res.text().await?;
        let url = regex::Regex::new(r#"Click <a href="(.*)">here</a>"#)
            .unwrap()
            .captures(&body)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
        let ticket = url.split("=").last().unwrap();
        dp!(ticket);

        dp!("--verify ticket");
        self.session.get(url).send().await?;

        dp!("--login to sis");
        let res = self
            .session
            .post("https://sis-eportal.hku.hk/psp/ptlprod/?cmd=login&languageCd=ENG")
            .form(&[
                ("ticket", ticket),
                ("userid", "hku_dummy"),
                ("pwd", "d"),
                ("timezoneOffset", "0"),
            ])
            .send()
            .await?;

        dp!("--checking login status");
        let body = res.text().await?;
        if body.contains("PSPAGE homePageHdr") {
            dp!("!-login to portal success");
            Ok(&self)
        } else {
            dp!("!-login to portal failed");
            Err("login to portal failed".into())
        }
    }

    pub async fn login_lib(
        &self,
        uid: &str,
        password: &str,
    ) -> Result<&Self, Box<dyn std::error::Error>> {
        dp!("-start login to library");
        dp!("--get lib login page");
        self.session
            .get("https://booking.lib.hku.hk/Secure/FacilityStatusDate.aspx")
            .send()
            .await?;

        let res = self.session.get("https://lib.hku.hk/hkulauth/legacy/authMain?uri=https://booking.lib.hku.hk/getpatron.aspx")
            .send().await?;

        dp!("--get scope and saml url");
        let body = res.text().await?;
        let scope = regex::Regex::new(r#"scope = "(.*)""#)
            .unwrap()
            .captures(&body)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
        let saml_url =
            regex::Regex::new(r#"<script src="(https://ids.hku.hk/idp/profile/SAML2.*)""#)
                .unwrap()
                .captures(&body)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str();

        dp!(scope);
        dp!(saml_url);

        dp!("--get saml page");
        self.session.get(saml_url).send().await?;

        let login_data = [
            ("conversation", "e1s1"),
            ("scope", scope),
            ("userid", uid),
            ("password", password),
            ("submit", "Submit"),
        ];

        dp!("--send login data");
        let res = self
            .session
            .post("https://ids.hku.hk/idp/ProcessAuthnLib")
            .form(&login_data)
            .send()
            .await?;


        dp!("--get saml data");
        let body = res.text().await?;
        let saml_response =
            regex::Regex::new(r#"<input type="hidden" name="SAMLResponse" value="(.*)"/>"#)
                .unwrap()
                .captures(&body)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str();
        let saml_data = [("SAMLResponse", saml_response), ("RelayState", scope)];

        dp!("--handle saml");
        let res = self
            .session
            .post("https://lib.hku.hk/hkulauth/handleSAML")
            .form(&saml_data)
            .send()
            .await?;

        dp!("--check login status");
        let body = res.text().await?;
        if body.contains("By making a booking / application, you are deemed to accept the relevant")
        {
            Err("login to library failed".into())
        } else {
            Ok(&self)
        }
    }

    pub async fn login_moodle(
        &self,
        uid: &str,
        password: &str,
    ) -> Result<&Self, Box<dyn std::error::Error>> {
        // TODO: login moodle
        dp!("-start login to moodle");

        dp!("--get login page");
        self.session.get("https://moodle.hku.hk/my/").send().await?;

        dp!("--goto hku login page");
        self.session
            .get("https://moodle.hku.hk/login/index.php?authCAS=CAS")
            .send()
            .await?;

        dp!("--generate keyid");
        let keyid = Local::now()
            .format("%Y%m%d%H%M%S")
            .to_string();
        dp!(&keyid);

        dp!("--send login data");
        let res = self.session
            .post("https://hkuportal.hku.hk/cas/servlet/edu.yale.its.tp.cas.servlet.Login")
            .form(&[
                ("keyid", keyid.as_str()),
                (
                    "service",
                    "https://moodle.hku.hk/login/index.php?authCAS=CAS",
                ),
                ("username", uid),
                ("password", password),
            ])
            .send()
            .await?;
        
        dp!("--get ticket");
        let body = res.text().await?;
        let url = regex::Regex::new(r#"Click <a href="(.*)">here</a>"#)
            .unwrap()
            .captures(&body)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
        let ticket = url.split("=").last().unwrap();
        dp!(ticket);

        dp!("--verify ticket");
        let res = self.session.get(url).send().await?;

        dp!("--check login status");
        let body = res.text().await?;
        if body.contains("My courses") {
            dp!("!-login to moodle success");
            Ok(&self)
        } else {
            dp!("!-login to moodle failed");
            Err("login to moodle failed".into())
        }
    }
}

impl Deref for Client {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn test_portal_login() {
        let uid = std::env::var("HKU_UID").unwrap_or_else(|_e| panic!("HKU_UID not set"));
        let pwd = std::env::var("HKU_PWD").unwrap_or_else(|_e| panic!("HKU_PWD not set"));
        let client = Client::new();
        aw!(client.login_portal(&uid, &pwd)).unwrap();
    }

    #[test]
    fn test_lib_login() {
        let uid = std::env::var("HKU_UID").unwrap_or_else(|_e| panic!("HKU_UID not set"));
        let pwd = std::env::var("HKU_PWD").unwrap_or_else(|_e| panic!("HKU_PWD not set"));
        let client = Client::new();
        aw!(client.login_lib(&uid, &pwd)).unwrap();
    }

    #[test]
    fn test_moodle_login() {
        let uid = std::env::var("HKU_UID").unwrap_or_else(|_e| panic!("HKU_UID not set"));
        let pwd = std::env::var("HKU_PWD").unwrap_or_else(|_e| panic!("HKU_PWD not set"));
        let client = Client::new();
        aw!(client.login_moodle(&uid, &pwd)).unwrap();
    }

    #[test]
    fn list_env_vars() {
        for (key, value) in std::env::vars() {
            println!("{}: {}", key, value);
        }
    }
}
