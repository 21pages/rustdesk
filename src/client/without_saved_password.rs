use super::{send_login, Interface, LoginConfigHandler};
use hbb_common::{
    sha2::{Digest, Sha256},
    Stream,
};
use std::sync::{Arc, RwLock};

impl LoginConfigHandler {
    pub(crate) fn enable_skip_saved_password(&mut self) {
        self.skip_saved_password = true;
        self.password.clear();
        self.shared_password = None;
        self.password_source = Default::default();
        self.remember = false;
    }
}

fn login_password(lc: &LoginConfigHandler) -> Vec<u8> {
    if lc.password.is_empty() {
        return Vec::new();
    }
    let mut hasher = Sha256::new();
    hasher.update(&lc.password);
    hasher.update(&lc.hash.challenge);
    hasher.finalize().to_vec()
}

pub(super) async fn handle_hash(
    lc: Arc<RwLock<LoginConfigHandler>>,
    interface: &impl Interface,
    peer: &mut Stream,
) -> bool {
    let password = login_password(&lc.read().unwrap());
    if password.is_empty() {
        interface.msgbox("input-password", "Password Required", "", "");
    }
    send_login(lc, String::new(), String::new(), password, peer).await;
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
        if !password.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(password);
            hasher.update(&lc.hash.salt);
            lc.password = hasher.finalize().to_vec();
        }
        login_password(&lc)
    };
    send_login(lc, os_username, os_password, password, peer).await;
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
                        lc.enable_skip_saved_password();
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
            lc.enable_skip_saved_password();
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
        assert!(received_password(&mut peer).await.is_empty());
    }
}
