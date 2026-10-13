use super::{send_login, Interface, LoginConfigHandler};
use base::message_proto::LoginMethods;
use hbb_common::{
    sha2::{Digest, Sha256},
    Stream,
};
use serde_json::json;
use std::sync::{Arc, RwLock};

#[derive(Default, PartialEq, Eq)]
pub(super) enum LoginMethod {
    #[default]
    Automatic,
    Consent,
    Password,
}

impl LoginConfigHandler {
    pub(crate) fn set_login_method(&mut self, method: &str) -> bool {
        self.login_method = match method {
            "consent" => LoginMethod::Consent,
            "password" => LoginMethod::Password,
            _ => return false,
        };
        self.password.clear();
        self.shared_password = None;
        self.password_source = Default::default();
        self.remember = false;
        true
    }

    pub(crate) fn skips_saved_password(&self) -> bool {
        self.login_method != LoginMethod::Automatic
    }
}

fn login_password(lc: &LoginConfigHandler) -> Vec<u8> {
    if lc.password.is_empty() || lc.login_method == LoginMethod::Consent {
        return Vec::new();
    }
    let mut hasher = Sha256::new();
    hasher.update(&lc.password);
    hasher.update(&lc.hash.challenge);
    hasher.finalize().to_vec()
}

fn show_dialog(lc: &Arc<RwLock<LoginConfigHandler>>, interface: &impl Interface, error: &str) {
    let data = {
        let lc = lc.read().unwrap();
        let methods = lc.hash.login_methods.as_ref();
        json!({
            "method": if lc.login_method == LoginMethod::Consent { "consent" } else { "password" },
            "click": methods.map(|m| m.click),
            "password": methods.map(|m| m.password),
            "error": error,
        })
    };
    interface.msgbox("login-method", "Connect", &data.to_string(), "");
}

pub(super) async fn handle_hash(
    lc: Arc<RwLock<LoginConfigHandler>>,
    interface: &impl Interface,
    peer: &mut Stream,
) -> bool {
    let (password, send, available) = {
        let lc = lc.read().unwrap();
        let methods = lc.hash.login_methods.as_ref();
        let password = login_password(&lc);
        let send = match lc.login_method {
            LoginMethod::Consent => methods.map_or(true, |m| m.click),
            LoginMethod::Password => methods.map_or(true, |m| m.password) && !password.is_empty(),
            LoginMethod::Automatic => return true,
        };
        (
            password,
            send,
            methods.map_or(true, |m| m.click || m.password),
        )
    };
    if !available {
        interface.msgbox(
            "error",
            "Login Error",
            "No authentication method is available on the remote device.",
            "",
        );
        return false;
    }
    if password.is_empty() || !send {
        show_dialog(&lc, interface, "");
    }
    if send {
        send_login(lc, String::new(), String::new(), password, peer).await;
    }
    true
}

pub(super) async fn handle_login_from_ui(
    lc: Arc<RwLock<LoginConfigHandler>>,
    os_username: String,
    os_password: String,
    password: String,
    peer: &mut Stream,
) {
    let password = {
        let mut lc = lc.write().unwrap();
        let methods = lc.hash.login_methods.as_ref();
        match lc.login_method {
            LoginMethod::Consent if methods.map_or(true, |m| m.click) => Vec::new(),
            LoginMethod::Password
                if methods.map_or(true, |m| m.password) && !password.is_empty() =>
            {
                let mut hasher = Sha256::new();
                hasher.update(password);
                hasher.update(&lc.hash.salt);
                lc.password = hasher.finalize().to_vec();
                login_password(&lc)
            }
            _ => return,
        }
    };
    send_login(lc, os_username, os_password, password, peer).await;
}

pub(super) fn handle_login_error(
    lc: &Arc<RwLock<LoginConfigHandler>>,
    err: &str,
    interface: &impl Interface,
) -> Option<bool> {
    {
        let mut lc = lc.write().unwrap();
        match err {
            super::LOGIN_MSG_NO_PASSWORD_ACCESS => {
                lc.set_login_method("consent");
                lc.hash.login_methods = Some(LoginMethods {
                    click: true,
                    ..Default::default()
                })
                .into();
            }
            super::LOGIN_MSG_PASSWORD_WRONG | super::LOGIN_MSG_PASSWORD_EMPTY => {
                lc.password.clear()
            }
            _ => return None,
        }
    }
    show_dialog(
        lc,
        interface,
        if err == super::LOGIN_MSG_NO_PASSWORD_ACCESS {
            ""
        } else {
            err
        },
    );
    Some(true)
}

#[cfg(all(test, feature = "flutter"))]
mod tests {
    use super::*;
    use crate::{client, flutter::FlutterHandler, ui_session_interface::Session};
    use base::message_proto::{Hash, Message, PeerInfo};
    use hbb_common::{
        config::PeerConfig, protobuf::Message as _, rendezvous_proto::ConnType, tcp::FramedStream,
        tokio,
    };

    async fn connection() -> (Stream, Stream) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = tokio::net::TcpStream::connect(addr).await.unwrap();
        let (socket, addr) = listener.accept().await.unwrap();
        (
            Stream::Tcp(FramedStream::from(client, listener.local_addr().unwrap())),
            Stream::Tcp(FramedStream::from(socket, addr)),
        )
    }

    async fn received_password(peer: &mut Stream) -> Vec<u8> {
        let bytes = peer.next_timeout(3000).await.unwrap().unwrap();
        Message::parse_from_bytes(&bytes)
            .unwrap()
            .login_request()
            .password
            .to_vec()
    }

    #[tokio::test]
    async fn configured_passwords_are_skipped_only_for_the_selected_connection() {
        for conn_type in [
            ConnType::DEFAULT_CONN,
            ConnType::FILE_TRANSFER,
            ConnType::TERMINAL,
        ] {
            for skip in [false, true] {
                let session = Session::<FlutterHandler>::default();
                {
                    let mut lc = session.lc.write().unwrap();
                    lc.id = format!("skip-saved-password-test-{}", uuid::Uuid::new_v4());
                    lc.conn_type = conn_type;
                    lc.config.password = vec![1; 32];
                    lc.password = vec![2; 32];
                    lc.shared_password = Some("shared password".into());
                    if skip {
                        assert!(lc.set_login_method("consent"));
                    }
                }
                let (mut stream, mut peer) = connection().await;
                assert!(
                    client::handle_hash(
                        session.lc.clone(),
                        "preset password",
                        Hash {
                            salt: "salt".into(),
                            challenge: "challenge".into(),
                            ..Default::default()
                        },
                        &session,
                        &mut stream,
                    )
                    .await
                );
                let password = received_password(&mut peer).await;
                assert_eq!(password.is_empty(), skip);
            }
        }
    }

    #[tokio::test]
    async fn one_time_login_preserves_saved_password_and_retries_without_it() {
        let (mut stream, mut peer) = connection().await;
        let id = format!("skip-saved-password-test-{}", uuid::Uuid::new_v4());
        let mut config = PeerConfig::default();
        config.password = vec![3; 32];
        config.store(&id);
        let session = Session::<FlutterHandler>::default();
        {
            let mut lc = session.lc.write().unwrap();
            lc.id = id.clone();
            lc.config = config.clone();
            assert!(lc.set_login_method("password"));
            lc.hash = Hash {
                salt: "salt".into(),
                challenge: "challenge".into(),
                ..Default::default()
            };
        }
        session
            .lc
            .write()
            .unwrap()
            .handle_peer_info(&PeerInfo::default());
        let saved_after_accept = PeerConfig::load(&id).password;
        client::handle_login_from_ui(
            session.lc.clone(),
            String::new(),
            String::new(),
            "one time password".into(),
            true,
            &mut stream,
        )
        .await;
        let first = received_password(&mut peer).await;
        assert!(!first.is_empty());
        session
            .lc
            .write()
            .unwrap()
            .handle_peer_info(&PeerInfo::default());
        let saved = PeerConfig::load(&id).password;
        PeerConfig::remove(&id);
        assert_eq!(saved_after_accept, config.password);
        assert_eq!(saved, config.password);

        assert!(client::handle_login_error(
            session.lc.clone(),
            client::LOGIN_MSG_PASSWORD_WRONG,
            &session,
        ));
        client::handle_login_from_ui(
            session.lc.clone(),
            String::new(),
            String::new(),
            String::new(),
            false,
            &mut stream,
        )
        .await;
        assert!(peer.next_timeout(50).await.is_none());
    }

    #[tokio::test]
    async fn unavailable_methods_never_send_a_login_or_fall_back_to_saved_passwords() {
        for (method, click, password) in [
            ("consent", false, true),
            ("password", true, false),
            ("consent", false, false),
        ] {
            let session = Session::<FlutterHandler>::default();
            {
                let mut lc = session.lc.write().unwrap();
                lc.config.password = vec![1; 32];
                assert!(lc.set_login_method(method));
            }
            let (mut stream, mut peer) = connection().await;
            let keep_alive = client::handle_hash(
                session.lc.clone(),
                "preset password",
                Hash {
                    login_methods: Some(LoginMethods {
                        click,
                        password,
                        ..Default::default()
                    })
                    .into(),
                    ..Default::default()
                },
                &session,
                &mut stream,
            )
            .await;
            assert_eq!(keep_alive, click || password);
            client::handle_login_from_ui(
                session.lc.clone(),
                String::new(),
                String::new(),
                "manual password".into(),
                false,
                &mut stream,
            )
            .await;
            assert!(peer.next_timeout(50).await.is_none());
        }
    }
}
