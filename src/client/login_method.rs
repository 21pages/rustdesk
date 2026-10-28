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
}

impl LoginConfigHandler {
    pub(crate) fn set_login_method(&mut self, method: &str) -> bool {
        self.login_method = match method {
            "consent" => LoginMethod::Consent,
            _ => return false,
        };
        true
    }

    pub(crate) fn skips_saved_password(&self) -> bool {
        self.login_method != LoginMethod::Automatic
    }
}

fn show_dialog(lc: &Arc<RwLock<LoginConfigHandler>>, interface: &impl Interface, error: &str) {
    let data = {
        let lc = lc.read().unwrap();
        let methods = lc.hash.login_methods.as_ref();
        json!({
            "click": methods.map(|m| m.click),
            "password": methods.map(|m| m.password),
            "os_account_required": lc.is_terminal_admin && error != super::LOGIN_MSG_NO_PASSWORD_ACCESS,
            "error": if error == super::LOGIN_MSG_NO_PASSWORD_ACCESS { "" } else { error },
        })
    };
    interface.msgbox("login-method", "Connect", &data.to_string(), "");
}

pub(super) fn handle_hash(
    lc: &Arc<RwLock<LoginConfigHandler>>,
    interface: &impl Interface,
) -> Option<bool> {
    let send = {
        let lc = lc.read().unwrap();
        let methods = lc.hash.login_methods.as_ref();
        methods.map_or(true, |m| m.click) && !lc.is_terminal_admin
    };
    show_dialog(lc, interface, "");
    if send {
        None
    } else {
        Some(true)
    }
}

pub(super) async fn handle_login_from_ui(
    lc: Arc<RwLock<LoginConfigHandler>>,
    os_username: String,
    os_password: String,
    password: String,
    peer: &mut Stream,
) {
    let password = {
        let lc = lc.read().unwrap();
        let methods = lc.hash.login_methods.as_ref();
        if lc.login_method != LoginMethod::Consent {
            return;
        }
        let mut challenge = Sha256::new();
        if methods.is_some_and(|m| !m.click) {
            if !methods.is_some_and(|m| m.password) || password.is_empty() {
                return;
            }
            let mut hasher = Sha256::new();
            hasher.update(password);
            hasher.update(&lc.hash.salt);
            challenge.update(hasher.finalize());
        } else {
            challenge.update(&lc.password);
        }
        challenge.update(&lc.hash.challenge);
        challenge.finalize().to_vec()
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
    show_dialog(lc, interface, err);
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

    #[test]
    fn session_creation_initializes_login_method() {
        let remote_id = hbb_common::config::LocalConfig::get_remote_id();
        for method in ["", "consent"] {
            let session_id = uuid::Uuid::new_v4();
            let result = crate::flutter_ffi::session_add_sync(
                session_id,
                format!("login-method-init-{}", session_id),
                false,
                false,
                false,
                false,
                false,
                String::new(),
                false,
                String::new(),
                false,
                None,
                method.to_owned(),
            );
            let session = crate::flutter::sessions::remove_session_by_session_id(&session_id);
            hbb_common::config::LocalConfig::set_remote_id(&remote_id);
            assert!(result.0.is_empty(), "{}", result.0);
            let session = session.unwrap();
            let request = session
                .lc
                .read()
                .unwrap()
                .create_login_msg(String::new(), String::new(), vec![1; 32]);
            assert_eq!(
                request.login_request().password.is_empty(),
                method == "consent"
            );
        }
    }

    #[tokio::test]
    async fn configured_passwords_are_skipped_only_for_the_selected_connection() {
        for conn_type in [
            ConnType::DEFAULT_CONN,
            ConnType::FILE_TRANSFER,
            ConnType::VIEW_CAMERA,
            ConnType::PORT_FORWARD,
            ConnType::RDP,
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
                    let token = lc.get_conn_token();
                    if skip {
                        assert!(lc.set_login_method("consent"));
                    }
                    assert_eq!(lc.get_conn_token(), token);
                    let request = lc.create_login_msg(String::new(), String::new(), vec![4; 32]);
                    assert_eq!(request.login_request().password.is_empty(), skip);
                }
                let (mut stream, mut peer) = connection().await;
                assert!(
                    client::handle_hash(
                        session.lc.clone(),
                        "preset password",
                        Hash {
                            salt: "salt".into(),
                            challenge: "challenge".into(),
                            login_methods: Some(LoginMethods {
                                click: true,
                                password: true,
                                ..Default::default()
                            })
                            .into(),
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
    async fn admin_terminal_waits_for_os_credentials_before_requesting_consent() {
        let session = Session::<FlutterHandler>::default();
        {
            let mut lc = session.lc.write().unwrap();
            lc.conn_type = ConnType::TERMINAL;
            lc.is_terminal_admin = true;
            assert!(lc.set_login_method("consent"));
        }
        let (mut stream, mut peer) = connection().await;
        assert!(
            client::handle_hash(
                session.lc.clone(),
                "preset password",
                Hash::default(),
                &session,
                &mut stream,
            )
            .await
        );
        assert!(peer.next_timeout(50).await.is_none());
        client::handle_login_from_ui(
            session.lc.clone(),
            "administrator".into(),
            "os password".into(),
            String::new(),
            false,
            &mut stream,
        )
        .await;
        let bytes = peer.next_timeout(3000).await.unwrap().unwrap();
        let message = Message::parse_from_bytes(&bytes).unwrap();
        let login = message.login_request();
        assert!(login.password.is_empty());
        assert_eq!(login.os_login.username, "administrator");
        assert_eq!(login.os_login.password, "os password");
    }

    #[tokio::test]
    async fn consent_preserves_saved_password() {
        let (mut stream, mut peer) = connection().await;
        let id = format!("skip-saved-password-test-{}", uuid::Uuid::new_v4());
        let mut config = PeerConfig::default();
        config.password = vec![3; 32];
        config.store(&id);
        let session = Session::<FlutterHandler>::default();
        {
            let mut lc = session.lc.write().unwrap();
            lc.initialize(
                id.clone(),
                ConnType::DEFAULT_CONN,
                None,
                false,
                None,
                None,
                None,
                "",
            );
            assert!(!lc.set_login_method("password"));
            assert!(lc.set_login_method("consent"));
        }
        assert!(
            client::handle_hash(
                session.lc.clone(),
                "",
                Hash::default(),
                &session,
                &mut stream,
            )
            .await
        );
        let first = received_password(&mut peer).await;
        assert!(first.is_empty());
        session
            .lc
            .write()
            .unwrap()
            .handle_peer_info(&PeerInfo::default());
        let saved = PeerConfig::load(&id).password;
        PeerConfig::remove(&id);
        assert_eq!(saved, config.password);
    }

    #[tokio::test]
    async fn unavailable_login_methods_stop_both_connection_modes() {
        for consent in [false, true] {
            for methods in [None, Some(LoginMethods::default())] {
                let legacy = methods.is_none();
                let session = Session::<FlutterHandler>::default();
                {
                    let mut lc = session.lc.write().unwrap();
                    lc.id = format!("unavailable-login-methods-{}", uuid::Uuid::new_v4());
                    if consent {
                        assert!(lc.set_login_method("consent"));
                    }
                }
                let (mut stream, mut peer) = connection().await;
                let keep_alive = client::handle_hash(
                    session.lc.clone(),
                    "saved password",
                    Hash {
                        salt: "salt".into(),
                        challenge: "challenge".into(),
                        login_methods: methods.into(),
                        ..Default::default()
                    },
                    &session,
                    &mut stream,
                )
                .await;
                assert_eq!(keep_alive, legacy, "consent={consent}, legacy={legacy}");
                if legacy {
                    assert_eq!(received_password(&mut peer).await.is_empty(), consent);
                } else {
                    assert!(peer.next_timeout(50).await.is_none());
                }
            }
        }
    }

    #[tokio::test]
    async fn password_only_peer_requires_an_explicit_password() {
        for password in [true, false] {
            let session = Session::<FlutterHandler>::default();
            {
                let mut lc = session.lc.write().unwrap();
                lc.id = format!("password-only-test-{}", uuid::Uuid::new_v4());
                lc.config.password = vec![1; 32];
                assert!(lc.set_login_method("consent"));
            }
            let (mut stream, mut peer) = connection().await;
            let keep_alive = client::handle_hash(
                session.lc.clone(),
                "preset password",
                Hash {
                    salt: "salt".into(),
                    challenge: "challenge".into(),
                    login_methods: Some(LoginMethods {
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
            assert_eq!(keep_alive, password);
            assert!(peer.next_timeout(50).await.is_none());
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
            client::handle_login_from_ui(
                session.lc.clone(),
                String::new(),
                String::new(),
                "manual password".into(),
                false,
                &mut stream,
            )
            .await;
            if password {
                let salted = Sha256::digest(b"manual passwordsalt");
                let mut expected = Sha256::new();
                expected.update(salted);
                expected.update(b"challenge");
                assert_eq!(
                    received_password(&mut peer).await,
                    expected.finalize().to_vec()
                );
            } else {
                assert!(peer.next_timeout(50).await.is_none());
            }
        }
    }
}
