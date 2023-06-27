#![allow(dead_code)]

use asession::{Session, SessionBuilder};
use std::ops::Deref;
use std::path::PathBuf;

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

    pub async fn get_ticket(&self, uid: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        let res = self
            .session
            .post("https://hkuportal.hku.hk/cas/servlet/edu.yale.its.tp.cas.servlet.Login")
            .form(&[
                ("keyid", ""),
                ("username", uid),
                ("password", password),
            ])
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

    pub async fn login_portal(&self, uid: &str, password: &str) -> Result<&Self, Box<dyn std::error::Error>> {
        // pass uid and password
        let res = self
            .session
            .post("https://hkuportal.hku.hk/cas/servlet/edu.yale.its.tp.cas.servlet.Login")
            .form(&[
                ("keyid", ""),
                ("username", uid),
                ("password", password),
            ])
            .send()
            .await?;

        // get ticket and url
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

        // verify ticket
        self.session.get(url).send().await?;

        // login to sis
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

        // let cookie_store = self.client.get_cookie_store();
        // let cookie = cookie_store.lock().unwrap();
        // // print all cookies
        // cookie.iter_any().for_each(|c| {
        //     dp!(c);
        // });

        // check login status
        let body = res.text().await?;
        if body.contains("PSPAGE homePageHdr") {
            Ok(&self)
        } else {
            Err("login failed".into())
        }
    }

    pub async fn login_lib(&self, uid: &str, password: &str) -> Result<&Self, Box<dyn std::error::Error>> {
        self.session
            .get("https://booking.lib.hku.hk/Secure/FacilityStatusDate.aspx")
            .send()
            .await?;

        let res = self.session.get("https://lib.hku.hk/hkulauth/legacy/authMain?uri=https://booking.lib.hku.hk/getpatron.aspx")
            .send().await?;

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

        self.session.get(saml_url).send().await?;

        let login_data = [
            ("conversation", "e1s1"),
            ("scope", scope),
            ("userid", uid),
            ("password", password),
            ("submit", "Submit"),
        ];

        // send login data
        let res = self.session.post("https://ids.hku.hk/idp/ProcessAuthnLib")
            .form(&login_data)
            .send().await?;

        // get saml data
        let body = res.text().await?;
        let saml_response = regex::Regex::new(r#"<input type="hidden" name="SAMLResponse" value="(.*)"/>"#)
            .unwrap()
            .captures(&body)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
        let saml_data = [
            ("SAMLResponse", saml_response),
            ("RelayState", scope),
        ];

        // handleSAML
        let res = self.session.post("https://lib.hku.hk/hkulauth/handleSAML")
            .form(&saml_data)
            .send().await?;


        let body = res.text().await?;
        if body.contains("By making a booking / application, you are deemed to accept the relevant") {
            Err("login failed".into())
        } else {
            Ok(&self)
        }

    }

    pub async fn login_moodle(&self, uid: &str, password: &str) -> Result<&Self, Box<dyn std::error::Error>> {
        // TODO: login moodle

        let ticket = self.get_ticket(uid, password).await?;
        let res = self
            .get("https://moodle.hku.hk/login/index.php?authCAS=CAS&ticket=".to_string() + &ticket)
            .send()
            .await?;


        let body = res.text().await?;
        dp!(body);

        Ok(&self)
    }

    pub fn store_cookie(&self, path: PathBuf){
        self.session.store_cookie(path);
    }
}

impl Deref for Client {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.session
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     macro_rules! aw {
//         ($e:expr) => {
//             tokio_test::block_on($e)
//         };
//     }

//     #[test]
//     fn test_portal_login() {
//         let client = Client::new();
//         aw!(client.login_portal("uid", "pwd")).unwrap();
//     }

//     #[test]
//     fn test_lib_login() {
//         let client = Client::new();
//         aw!(client.login_lib("uid", "pwd")).unwrap();
//     }

//     #[test]
//     fn test_moodle_login() {
//         let client = Client::new();
//         aw!(client.login_moodle("uid", "pwd")).unwrap();
//     }
// }
