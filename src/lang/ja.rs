lazy_static::lazy_static! {
pub static ref T: std::collections::HashMap<&'static str, &'static str> =
    [
        ("Status", "状態"),
        ("Your Desktop", "デスクトップ"),
        ("desk_tip", "このIDとパスワードであなたのデスクトップにアクセスできます。"),
        ("Password", "パスワード"),
        ("Ready", "準備完了"),
        ("Established", "接続完了"),
        ("connecting_status", "RuskDeskネットワークに接続中..."),
        ("Enable Service", "サービスを有効化"),
        ("Start Service", "サービスを開始"),
        ("Service is running", "サービスは動作中"),
        ("Service is not running", "サービスは動作していません"),
        ("not_ready_status", "準備できていません。接続を確認してください。"),
        ("Control Remote Desktop", "リモートのデスクトップを操作する"),
        ("Transfer File", "ファイルを転送"),
        ("Connect", "接続"),
        ("Recent Sessions", "最近のセッション"),
        ("Address Book", "アドレス帳"),
        ("Confirmation", "確認用"),
        ("TCP Tunneling", "TCPトンネリング"),
        ("Remove", "削除"),
        ("Refresh random password", "ランダムパスワードを再生成"),
        ("Set your own password", "自分のパスワードを設定"),
        ("Enable Keyboard/Mouse", "キーボード・マウスを有効化"),
        ("Enable Clipboard", "クリップボードを有効化"),
        ("Enable File Transfer", "ファイル転送を有効化"),
        ("Enable TCP Tunneling", "TCPトンネリングを有効化"),
        ("IP Whitelisting", "IPホワイトリスト"),
        ("ID/Relay Server", "認証・中継サーバー"),
        ("Import Server Config", "サーバー設定をインポート"),
        ("Export Server Config", ""),
        ("Import server configuration successfully", "サーバー設定をインポートしました"),
        ("Export server configuration successfully", ""),
        ("Invalid server configuration", "無効なサーバー設定です"),
        ("Clipboard is empty", "クリップボードは空です"),
        ("Stop service", "サービスを停止"),
        ("Change ID", "IDを変更"),
        ("Your new ID", ""),
        ("length %min% to %max%", ""),
        ("starts with a letter", ""),
        ("allowed characters", ""),
        ("id_change_tip", "使用できるのは大文字・小文字のアルファベット、数字、アンダースコア（_）のみです。初めの文字はアルファベットにする必要があります。6文字から16文字までです。"),
        ("Website", "公式サイト"),
        ("About", "情報"),
        ("Slogan_tip", ""),
        ("Privacy Statement", ""),
        ("Mute", "ミュート"),
        ("Build Date", ""),
        ("Version", ""),
        ("Home", ""),
        ("Audio Input", "音声入力デバイス"),
        ("Enhancements", "追加機能"),
        ("Hardware Codec", "ハードウェア コーデック"),
        ("Adaptive bitrate", "アダプティブビットレート"),
        ("ID Server", "認証サーバー"),
        ("Relay Server", "中継サーバー"),
        ("API Server", "APIサーバー"),
        ("invalid_http", "http:// もしくは https:// から入力してください"),
        ("Invalid IP", "無効なIP"),
        ("Invalid format", "無効な形式"),
        ("server_not_support", "サーバー側でまだサポートされていません"),
        ("Not available", "利用不可"),
        ("Too frequent", "使用量が多すぎです"),
        ("Cancel", "キャンセル"),
        ("Skip", "スキップ"),
        ("Close", "閉じる"),
        ("Retry", "再試行"),
        ("OK", "OK"),
        ("Password Required", "パスワードが必要"),
        ("Please enter your password", "パスワードを入力してください"),
        ("Remember password", "パスワードを記憶する"),
        ("Wrong Password", "パスワードが間違っています"),
        ("Do you want to enter again?", "もう一度入力しますか？"),
        ("Connection Error", "接続エラー"),
        ("Error", "エラー"),
        ("Reset by the peer", "相手がリセットしました"),
        ("Connecting...", "接続中..."),
        ("Connection in progress. Please wait.", "接続中です。しばらくお待ちください。"),
        ("Please try 1 minute later", "1分後にもう一度お試しください"),
        ("Login Error", "ログインエラー"),
        ("Successful", "成功"),
        ("Connected, waiting for image...", "接続完了、画像を取得中..."),
        ("Name", "名前"),
        ("Type", "種類"),
        ("Modified", "最終更新"),
        ("Size", "サイズ"),
        ("Show Hidden Files", "隠しファイルを表示"),
        ("Receive", "受信"),
        ("Send", "送信"),
        ("Refresh File", "ファイルを更新"),
        ("Local", "ローカル"),
        ("Remote", "リモート"),
        ("Remote Computer", "リモート側コンピューター"),
        ("Local Computer", "ローカル側コンピューター"),
        ("Confirm Delete", "削除の確認"),
        ("Delete", "削除"),
        ("Properties", "プロパティ"),
        ("Multi Select", "複数選択"),
        ("Select All", ""),
        ("Unselect All", ""),
        ("Empty Directory", "空のディレクトリ"),
        ("Not an empty directory", "空ではないディレクトリ"),
        ("Are you sure you want to delete this file?", "本当にこのファイルを削除しますか？"),
        ("Are you sure you want to delete this empty directory?", "本当にこの空のディレクトリを削除しますか？"),
        ("Are you sure you want to delete the file of this directory?", "本当にこのディレクトリ内のファイルを削除しますか？"),
        ("Do this for all conflicts", "他のすべてにも適用する"),
        ("This is irreversible!", "この操作は元に戻せません！"),
        ("Deleting", "削除中"),
        ("files", "ファイル"),
        ("Waiting", "待機中"),
        ("Finished", "完了"),
        ("Speed", "速度"),
        ("Custom Image Quality", "画質を調整"),
        ("Privacy mode", "プライバシーモード"),
        ("Block user input", "ユーザーの入力をブロック"),
        ("Unblock user input", "ユーザーの入力を許可"),
        ("Adjust Window", "ウィンドウを調整"),
        ("Original", "オリジナル"),
        ("Shrink", "縮小"),
        ("Stretch", "伸縮"),
        ("Scrollbar", ""),
        ("ScrollAuto", ""),
        ("Good image quality", "画質優先"),
        ("Balanced", "バランス"),
        ("Optimize reaction time", "速度優先"),
        ("Custom", ""),
        ("Show remote cursor", "リモート側のカーソルを表示"),
        ("Show quality monitor", "品質モニターを表示"),
        ("Disable clipboard", "クリップボードを無効化"),
        ("Lock after session end", "セッション終了後にロックする"),
        ("Insert", "送信"),
        ("Insert Lock", "ロック命令を送信"),
        ("Refresh", "更新"),
        ("ID does not exist", "IDが存在しません"),
        ("Failed to connect to rendezvous server", "ランデブーサーバーに接続できませんでした"),
        ("Please try later", "後でもう一度お試しください"),
        ("Remote desktop is offline", "リモート側デスクトップがオフラインです"),
        ("Key mismatch", "キーが一致しません"),
        ("Timeout", "タイムアウト"),
        ("Failed to connect to relay server", "中継サーバーに接続できませんでした"),
        ("Failed to connect via rendezvous server", "ランデブーサーバー経由で接続できませんでした"),
        ("Failed to connect via relay server", "中継サーバー経由で接続できませんでした"),
        ("Failed to make direct connection to remote desktop", "リモート側デスクトップと直接接続できませんでした"),
        ("Set Password", "パスワードを設定"),
        ("OS Password", "OSのパスワード"),
        ("install_tip", "RustDeskがUACの影響によりリモート側で正常に動作しない場合があります。UACを回避するには、下のボタンをクリックしてシステムにRustDeskをインストールしてください。"),
        ("Click to upgrade", "アップグレード"),
        ("Click to download", "ダウンロード"),
        ("Click to update", "アップデート"),
        ("Configure", "設定"),
        ("config_acc", "リモートからあなたのデスクトップを操作するには、RustDeskに「アクセシビリティ」権限を与える必要があります。"),
        ("config_screen", "リモートからあなたのデスクトップにアクセスするには、RustDeskに「画面収録」権限を与える必要があります。"),
        ("Installing ...", "インストール中..."),
        ("Install", "インストール"),
        ("Installation", "インストール"),
        ("Installation Path", "インストール先のパス"),
        ("Create start menu shortcuts", "スタートメニューにショートカットを作成する"),
        ("Create desktop icon", "デスクトップにアイコンを作成する"),
        ("agreement_tip", "インストールを開始することで、ライセンス条項に同意したとみなされます。"),
        ("Accept and Install", "同意してインストール"),
        ("End-user license agreement", "エンドユーザー ライセンス条項"),
        ("Generating ...", "生成中 ..."),
        ("Your installation is lower version.", "インストール済みのバージョンが古いです。"),
        ("not_close_tcp_tip", "トンネルを使用中はこのウィンドウを閉じないでください"),
        ("Listening ...", "リッスン中 ..."),
        ("Remote Host", "リモートのホスト"),
        ("Remote Port", "リモートのポート"),
        ("Action", "操作"),
        ("Add", "追加"),
        ("Local Port", "ローカルのポート"),
        ("Local Address", ""),
        ("Change Local Port", ""),
        ("setup_server_tip", "接続をより速くするには、自分のサーバーをセットアップしてください"),
        ("Too short, at least 6 characters.", "短すぎます。最低6文字です。"),
        ("The confirmation is not identical.", "確認用と一致しません。"),
        ("Permissions", "権限"),
        ("Accept", "承諾"),
        ("Dismiss", "無視"),
        ("Disconnect", "切断"),
        ("Allow using keyboard and mouse", "キーボード・マウスの使用を許可"),
        ("Allow using clipboard", "クリップボードの使用を許可"),
        ("Allow hearing sound", "サウンドの受信を許可"),
        ("Allow file copy and paste", "ファイルのコピーアンドペーストを許可"),
        ("Connected", "接続済み"),
        ("Direct and encrypted connection", "接続は暗号化され、直接つながっている"),
        ("Relayed and encrypted connection", "接続は暗号化され、中継されている"),
        ("Direct and unencrypted connection", "接続は暗号化されてなく、直接つながっている"),
        ("Relayed and unencrypted connection", "接続は暗号化されてなく、中継されている"),
        ("Enter Remote ID", "リモートのIDを入力"),
        ("Enter your password", "パスワードを入力"),
        ("Logging in...", "ログイン中..."),
        ("Enable RDP session sharing", "RDPセッション共有を有効化"),
        ("Auto Login", "自動ログイン"),
        ("Enable Direct IP Access", "直接IPアクセスを有効化"),
        ("Rename", "名前の変更"),
        ("Space", "スペース"),
        ("Create Desktop Shortcut", "デスクトップにショートカットを作成する"),
        ("Change Path", "パスを変更"),
        ("Create Folder", "フォルダを作成"),
        ("Please enter the folder name", "フォルダ名を入力してください"),
        ("Fix it", "修復"),
        ("Warning", "注意"),
        ("Login screen using Wayland is not supported", "Waylandを使用したログインスクリーンはサポートされていません"),
        ("Reboot required", "再起動が必要"),
        ("Unsupported display server", "サポートされていないディスプレイサーバー"),
        ("x11 expected", "X11 が必要です"),
        ("Port", ""),
        ("Settings", "設定"),
        ("Username", "ユーザー名"),
        ("Invalid port", "無効なポート"),
        ("Closed manually by the peer", "相手が手動で切断しました"),
        ("Enable remote configuration modification", "リモート設定変更を有効化"),
        ("Run without install", "インストールせずに実行"),
        ("Connect via relay", ""),
        ("Always connect via relay", "常に中継サーバー経由で接続"),
        ("whitelist_tip", "ホワイトリストに登録されたIPからのみ接続を許可します"),
        ("Login", "ログイン"),
        ("Verify", ""),
        ("Remember me", ""),
        ("Trust this device", ""),
        ("Verification code", ""),
        ("verification_tip", ""),
        ("Logout", "ログアウト"),
        ("Tags", "タグ"),
        ("Search ID", "IDを検索"),
        ("whitelist_sep", "カンマやセミコロン、空白、改行で区切ってください"),
        ("Add ID", "IDを追加"),
        ("Add Tag", "タグを追加"),
        ("Unselect all tags", "全てのタグを選択解除"),
        ("Network error", "ネットワークエラー"),
        ("Username missed", "ユーザー名がありません"),
        ("Password missed", "パスワードがありません"),
        ("Wrong credentials", "資格情報が間違っています"),
        ("The verification code is incorrect or has expired", ""),
        ("Edit Tag", "タグを編集"),
        ("Unremember Password", "パスワードの記憶を解除"),
        ("Favorites", "お気に入り"),
        ("Add to Favorites", "お気に入りに追加"),
        ("Remove from Favorites", "お気に入りから削除"),
        ("Empty", "空"),
        ("Invalid folder name", "無効なフォルダ名"),
        ("Socks5 Proxy", "SOCKS5プロキシ"),
        ("Hostname", "ホスト名"),
        ("Discovered", "探知済み"),
        ("install_daemon_tip", "起動時に開始するには、システムサービスをインストールする必要があります。"),
        ("Remote ID", "リモートのID"),
        ("Paste", "ペースト"),
        ("Paste here?", "ここにペースト？"),
        ("Are you sure to close the connection?", "本当に切断しますか？"),
        ("Download new version", "新しいバージョンをダウンロード"),
        ("Touch mode", "タッチモード"),
        ("Mouse mode", "マウスモード"),
        ("One-Finger Tap", "1本指でタップ"),
        ("Left Mouse", "マウス左クリック"),
        ("One-Long Tap", "1本指でロングタップ"),
        ("Two-Finger Tap", "2本指でタップ"),
        ("Right Mouse", "マウス右クリック"),
        ("One-Finger Move", "1本指でドラッグ"),
        ("Double Tap & Move", "2本指でタップ&ドラッグ"),
        ("Mouse Drag", "マウスドラッグ"),
        ("Three-Finger vertically", "3本指で縦方向"),
        ("Mouse Wheel", "マウスホイール"),
        ("Two-Finger Move", "2本指でドラッグ"),
        ("Canvas Move", "キャンバスの移動"),
        ("Pinch to Zoom", "ピンチしてズーム"),
        ("Canvas Zoom", "キャンバスのズーム"),
        ("Reset canvas", "キャンバスのリセット"),
        ("No permission of file transfer", "ファイル転送の権限がありません"),
        ("Note", "ノート"),
        ("Connection", "接続"),
        ("Share Screen", "画面を共有"),
        ("Chat", "チャット"),
        ("Total", "計"),
        ("items", "個のアイテム"),
        ("Selected", "選択済み"),
        ("Screen Capture", "画面キャプチャ"),
        ("Input Control", "入力操作"),
        ("Audio Capture", "音声キャプチャ"),
        ("File Connection", "ファイルの接続"),
        ("Screen Connection", "画面の接続"),
        ("Do you accept?", "承諾しますか？"),
        ("Open System Setting", "端末設定を開く"),
        ("How to get Android input permission?", "Androidの入力権限を取得するには？"),
        ("android_input_permission_tip1", "このAndroid端末をリモートの端末からマウスやタッチで操作するには、RustDeskに「アクセシビリティ」サービスの使用を許可する必要があります。"),
        ("android_input_permission_tip2", "次の端末設定ページに進み、「インストール済みアプリ」から「RestDesk Input」をオンにしてください。"),
        ("android_new_connection_tip", "新しい操作リクエストが届きました。この端末を操作しようとしています。"),
        ("android_service_will_start_tip", "「画面キャプチャ」をオンにするとサービスが自動的に開始され、他の端末がこの端末への接続をリクエストできるようになります。"),
        ("android_stop_service_tip", "サービスを停止すると、現在確立されている接続が全て自動的に閉じられます。"),
        ("android_version_audio_tip", "現在のAndroidバージョンでは音声キャプチャはサポートされていません。Android 10以降にアップグレードしてください。"),
        ("android_start_service_tip", ""),
        ("android_permission_may_not_change_tip", ""),
        ("Account", ""),
        ("Overwrite", "上書き"),
        ("This file exists, skip or overwrite this file?", "このファイルは存在しています。スキップするか上書きしますか？"),
        ("Quit", "終了"),
        ("doc_mac_permission", "https://rustdesk.com/docs/en/manual/mac/#enable-permissions"), // @TODO: Update url when someone translates the docum"),
        ("Help", "ヘルプ"),
        ("Failed", "失敗"),
        ("Succeeded", "成功"),
        ("Someone turns on privacy mode, exit", "プライバシーモードがオンになりました。終了します。"),
        ("Unsupported", "サポートされていません"),
        ("Peer denied", "相手が拒否しました"),
        ("Please install plugins", "プラグインをインストールしてください"),
        ("Peer exit", "相手が終了しました"),
        ("Failed to turn off", "オフにできませんでした"),
        ("Turned off", "オフになりました"),
        ("In privacy mode", "プライバシーモード開始"),
        ("Out privacy mode", "プライバシーモード終了"),
        ("Language", "言語"),
        ("Keep RustDesk background service", "RustDesk バックグラウンドサービスを維持"),
        ("Ignore Battery Optimizations", "バッテリーの最適化を無効にする"),
        ("android_open_battery_optimizations_tip", "この機能を使わない場合は、次のRestDeskアプリ設定ページから「バッテリー」に進み、「制限なし」の選択を外してください"),
        ("Start on Boot", ""),
        ("Start the screen sharing service on boot, requires special permissions", ""),
        ("Connection not allowed", "接続が許可されていません"),
        ("Legacy mode", ""),
        ("Map mode", ""),
        ("Translate mode", ""),
        ("Use permanent password", "固定のパスワードを使用"),
        ("Use both passwords", "どちらのパスワードも使用"),
        ("Set permanent password", "固定のパスワードを設定"),
        ("Enable Remote Restart", "リモートからの再起動を有効化"),
        ("Allow remote restart", "リモートからの再起動を許可"),
        ("Restart Remote Device", "リモートの端末を再起動"),
        ("Are you sure you want to restart", "本当に再起動しますか"),
        ("Restarting Remote Device", "リモート端末を再起動中"),
        ("remote_restarting_tip", "リモート端末は再起動中です。このメッセージボックスを閉じて、しばらくした後に固定のパスワードを使用して再接続してください。"),
        ("Copied", ""),
        ("Exit Fullscreen", "全画面表示を終了"),
        ("Fullscreen", "全画面表示"),
        ("Mobile Actions", "モバイル アクション"),
        ("Select Monitor", "モニターを選択"),
        ("Control Actions", "コントロール アクション"),
        ("Display Settings", "ディスプレイの設定"),
        ("Ratio", "比率"),
        ("Image Quality", "画質"),
        ("Scroll Style", "スクロール スタイル"),
        ("Show Toolbar", ""),
        ("Hide Toolbar", ""),
        ("Direct Connection", "直接接続"),
        ("Relay Connection", "リレー接続"),
        ("Secure Connection", "安全な接続"),
        ("Insecure Connection", "安全でない接続"),
        ("Scale original", "オリジナルサイズ"),
        ("Scale adaptive", "フィットウィンドウ"),
        ("General", ""),
        ("Security", ""),
        ("Theme", ""),
        ("Dark Theme", ""),
        ("Light Theme", ""),
        ("Dark", ""),
        ("Light", ""),
        ("Follow System", ""),
        ("Enable hardware codec", ""),
        ("Unlock Security Settings", ""),
        ("Enable Audio", ""),
        ("Unlock Network Settings", ""),
        ("Server", ""),
        ("Direct IP Access", ""),
        ("Proxy", ""),
        ("Apply", ""),
        ("Disconnect all devices?", ""),
        ("Clear", ""),
        ("Audio Input Device", ""),
        ("Use IP Whitelisting", ""),
        ("Network", ""),
        ("Enable RDP", ""),
        ("Pin Toolbar", ""),
        ("Unpin Toolbar", ""),
        ("Recording", ""),
        ("Directory", ""),
        ("Automatically record incoming sessions", ""),
        ("Change", ""),
        ("Start session recording", ""),
        ("Stop session recording", ""),
        ("Enable Recording Session", ""),
        ("Allow recording session", ""),
        ("Enable LAN Discovery", ""),
        ("Deny LAN Discovery", ""),
        ("Write a message", ""),
        ("Prompt", ""),
        ("Please wait for confirmation of UAC...", ""),
        ("elevated_foreground_window_tip", ""),
        ("Disconnected", ""),
        ("Other", "他の"),
        ("Confirm before closing multiple tabs", "同時に複数のタブを閉じる前に確認する"),
        ("Keyboard Settings", ""),
        ("Full Access", ""),
        ("Screen Share", ""),
        ("Wayland requires Ubuntu 21.04 or higher version.", "Wayland には、Ubuntu 21.04 以降のバージョンが必要です。"),
        ("Wayland requires higher version of linux distro. Please try X11 desktop or change your OS.", "Wayland には、より高いバージョンの Linux ディストリビューションが必要です。 X11 デスクトップを試すか、OS を変更してください。"),
        ("JumpLink", "View"),
        ("Please Select the screen to be shared(Operate on the peer side).", "共有する画面を選択してください(ピア側で操作)。"),
        ("Show RustDesk", ""),
        ("This PC", ""),
        ("or", ""),
        ("Continue with", ""),
        ("Elevate", ""),
        ("Zoom cursor", ""),
        ("Accept sessions via password", ""),
        ("Accept sessions via click", ""),
        ("Accept sessions via both", ""),
        ("Please wait for the remote side to accept your session request...", ""),
        ("One-time Password", ""),
        ("Use one-time password", ""),
        ("One-time password length", ""),
        ("Request access to your device", ""),
        ("Hide connection management window", ""),
        ("hide_cm_tip", ""),
        ("wayland_experiment_tip", ""),
        ("Right click to select tabs", ""),
        ("Skipped", ""),
        ("Add to Address Book", ""),
        ("Group", ""),
        ("Search", ""),
        ("Closed manually by web console", ""),
        ("Local keyboard type", ""),
        ("Select local keyboard type", ""),
        ("software_render_tip", ""),
        ("Always use software rendering", ""),
        ("config_input", ""),
        ("config_microphone", ""),
        ("request_elevation_tip", ""),
        ("Wait", ""),
        ("Elevation Error", ""),
        ("Ask the remote user for authentication", ""),
        ("Choose this if the remote account is administrator", ""),
        ("Transmit the username and password of administrator", ""),
        ("still_click_uac_tip", ""),
        ("Request Elevation", ""),
        ("wait_accept_uac_tip", ""),
        ("Elevate successfully", ""),
        ("uppercase", ""),
        ("lowercase", ""),
        ("digit", ""),
        ("special character", ""),
        ("length>=8", ""),
        ("Weak", ""),
        ("Medium", ""),
        ("Strong", ""),
        ("Switch Sides", ""),
        ("Please confirm if you want to share your desktop?", ""),
        ("Display", ""),
        ("Default View Style", ""),
        ("Default Scroll Style", ""),
        ("Default Image Quality", ""),
        ("Default Codec", ""),
        ("Bitrate", ""),
        ("FPS", ""),
        ("Auto", ""),
        ("Other Default Options", ""),
        ("Voice call", ""),
        ("Text chat", ""),
        ("Stop voice call", ""),
        ("relay_hint_tip", ""),
        ("Reconnect", ""),
        ("Codec", ""),
        ("Resolution", ""),
        ("No transfers in progress", ""),
        ("Set one-time password length", ""),
        ("install_cert_tip", ""),
        ("confirm_install_cert_tip", ""),
        ("RDP Settings", ""),
        ("Sort by", ""),
        ("New Connection", ""),
        ("Restore", ""),
        ("Minimize", ""),
        ("Maximize", ""),
        ("Your Device", ""),
        ("empty_recent_tip", ""),
        ("empty_favorite_tip", ""),
        ("empty_lan_tip", ""),
        ("empty_address_book_tip", ""),
        ("eg: admin", ""),
        ("Empty Username", ""),
        ("Empty Password", ""),
        ("Me", ""),
        ("identical_file_tip", ""),
        ("show_monitors_tip", ""),
        ("View Mode", ""),
        ("login_linux_tip", ""),
        ("verify_rustdesk_password_tip", ""),
        ("remember_account_tip", ""),
        ("os_account_desk_tip", ""),
        ("OS Account", ""),
        ("another_user_login_title_tip", ""),
        ("another_user_login_text_tip", ""),
        ("xorg_not_found_title_tip", ""),
        ("xorg_not_found_text_tip", ""),
        ("no_desktop_title_tip", ""),
        ("no_desktop_text_tip", ""),
        ("No need to elevate", ""),
        ("System Sound", ""),
        ("Default", ""),
        ("New RDP", ""),
        ("Fingerprint", ""),
        ("Copy Fingerprint", ""),
        ("no fingerprints", ""),
        ("Select a peer", ""),
        ("Select peers", ""),
        ("Plugins", ""),
        ("Uninstall", ""),
        ("Update", ""),
        ("Enable", ""),
        ("Disable", ""),
        ("Options", ""),
        ("resolution_original_tip", ""),
        ("resolution_fit_local_tip", ""),
        ("resolution_custom_tip", ""),
        ("Collapse toolbar", ""),
        ("Accept and Elevate", ""),
        ("accept_and_elevate_btn_tooltip", ""),
        ("clipboard_wait_response_timeout_tip", ""),
        ("Incoming connection", ""),
        ("Outgoing connection", ""),
        ("Exit", ""),
        ("Open", ""),
        ("logout_tip", ""),
        ("Service", ""),
        ("Start", ""),
        ("Stop", ""),
        ("exceed_max_devices", ""),
        ("Sync with recent sessions", ""),
        ("Sort tags", ""),
        ("Open connection in new tab", ""),
        ("Move tab to new window", ""),
        ("Can not be empty", ""),
        ("Already exists", ""),
        ("Change Password", ""),
        ("Refresh Password", ""),
        ("ID", ""),
        ("Grid View", ""),
        ("List View", ""),
        ("Select", ""),
        ("Toggle Tags", ""),
        ("pull_ab_failed_tip", ""),
        ("push_ab_failed_tip", ""),
        ("synced_peer_readded_tip", ""),
        ("Change Color", ""),
        ("Primary Color", ""),
        ("HSV Color", ""),
        ("Installation Successful!", ""),
        ("Installation failed!", ""),
        ("controlled sessions", ""),
    ].iter().cloned().collect();
}
