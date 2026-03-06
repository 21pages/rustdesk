# EASY_ACCESS_CLAUDE.md

Easy Access = passwordless login via Ed25519 challenge-response.

## Current Flow

1. Controller prepares a persistent user keypair in `LocalConfig`:
- `easy-access-user-public-key`
- `easy-access-user-secret-key`

2. Controller sends `easyAccessPublicKey` in auth payloads:
- `/api/login` (account + email_code/2fa flows)
- `/api/currentUser` refresh
- `/api/oidc/auth-query` (query param: `easy_access_public_key`)

3. HBBS stores PK in session (`SessionInfo.easy_access_public_key`) and checks ACL.

4. Rendezvous marks `easy_access_granted` in:
- `ControlledConfig.easy_access_granted` (to controlled)
- `ControllerConfig.easy_access_granted` (to controller)

5. Controlled generates challenge (32 bytes), controller signs with SK, sends:
- `LoginRequest.easy_access_public_key`
- `LoginRequest.easy_access_signature`

6. Controlled verifies:
- public key is in locally approved list
- signature is valid for challenge

## Important Notes

- No separate “upload pk after login” step anymore.
- `POST /api/easy-access/session-public-key` was removed.
- `GET /api/easy-access/authorized-keys/:peer_id` is still used for controlled-side manual sync/approval.

## Key Files

### rustdesk

- `src/easy_access_keys.rs`
  - keypair load/create from `LocalConfig`
  - challenge signing
- `src/ui_interface.rs`
  - `get_local_option("easy-access-user-public-key")` auto-generates if missing
- `flutter/lib/common/hbbs/hbbs.dart`
  - `LoginRequest.toJson()` injects `easyAccessPublicKey`
- `flutter/lib/models/user_model.dart`
  - `refreshCurrentUser()` includes `easyAccessPublicKey`
- `src/ui/index.tis`
  - sciter login/refresh includes `easyAccessPublicKey`
- `src/hbbs_http/account.rs`
  - OIDC auth-query includes `easy_access_public_key`
- `src/client.rs`
  - uses `easy_access_granted` and challenge for login signing
- `src/server/connection.rs`
  - verifies signature against approved key list

### hbbs

- `src/session.rs`
  - `SessionInfo.easy_access_public_key`
- `src/auth.rs`
  - consumes pk from login/currentUser/oidc_auth_query payloads
  - writes pk into session
- `src/easy_access.rs`
  - `check_easy_access()`
  - ACL evaluation + authorized key collection
- `src/control_role.rs`
  - sets `ControlledConfig.easy_access_granted`
- `src/rendezvous_server.rs`
  - propagates easy-access grant to controller responses
- `src/api.rs`
  - only `GET /api/easy-access/authorized-keys/:peer_id` remains for sync

## Security Model

- Server can decide ACL but cannot forge signature without user SK.
- Controlled side trust anchor is local approved-key list (manual admin approval).
