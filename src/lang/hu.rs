lazy_static::lazy_static! {
pub static ref T: std::collections::HashMap<&'static str, &'static str> =
    [
        ("Status", "Státusz"),
        ("Your Desktop", "Saját azonosító"),
        ("desk_tip", "A számítógép ezzel a jelszóval és azonosítóval érhető el távolról."),
        ("Password", "Jelszó"),
        ("Ready", "Kész"),
        ("Established", "Létrejött"),
        ("connecting_status", "Csatlakozás folyamatban..."),
        ("Enable Service", "Szolgáltatás engedélyezése"),
        ("Start Service", "Szolgáltatás indítása"),
        ("Service is running", "Szolgáltatás aktív"),
        ("Service is not running", "Szolgáltatás inaktív"),
        ("not_ready_status", "Kapcsolódási hiba. Kérlek ellenőrizze a hálózati beállításokat."),
        ("Control Remote Desktop", "Távoli számítógép vezérlése"),
        ("Transfer File", "Fájlátvitel"),
        ("Connect", "Csatlakozás"),
        ("Recent Sessions", "Legutóbbi munkamanetek"),
        ("Address Book", "Címjegyzék"),
        ("Confirmation", "Megerősítés"),
        ("TCP Tunneling", "TCP Tunneling"),
        ("Remove", "Eltávolít"),
        ("Refresh random password", "Új véletlenszerű jelszó"),
        ("Set your own password", "Saját jelszó beállítása"),
        ("Enable Keyboard/Mouse", "Billentyűzet/egér engedélyezése"),
        ("Enable Clipboard", "Megosztott vágólap engedélyezése"),
        ("Enable File Transfer", "Fájlátvitel engedélyezése"),
        ("Enable TCP Tunneling", "TCP Tunneling engedélyezése"),
        ("IP Whitelisting", "IP engedélyezési lista"),
        ("ID/Relay Server", "Kiszolgáló szerver"),
        ("Import Server Config", "Szerver konfiguráció importálása"),
        ("Export Server Config", "Szerver konfiguráció exportálása"),
        ("Import server configuration successfully", "Szerver konfiguráció sikeresen importálva"),
        ("Export server configuration successfully", "Szerver konfiguráció sikeresen exportálva"),
        ("Invalid server configuration", "Érvénytelen szerver konfiguráció"),
        ("Clipboard is empty", "A vágólap üres"),
        ("Stop service", "Szolgáltatás leállítása"),
        ("Change ID", "Azonosító megváltoztatása"),
        ("Your new ID", ""),
        ("length %min% to %max%", ""),
        ("starts with a letter", ""),
        ("allowed characters", ""),
        ("id_change_tip", "Csak a-z, A-Z, 0-9 csoportokba tartozó karakterek, illetve a _ karakter van engedélyezve. Az első karakternek mindenképpen a-z, A-Z csoportokba kell esnie. Az azonosító hosszúsága 6-tól, 16 karakter."),
        ("Website", "Weboldal"),
        ("About", "Rólunk"),
        ("Slogan_tip", ""),
        ("Privacy Statement", ""),
        ("Mute", "Némítás"),
        ("Build Date", ""),
        ("Version", ""),
        ("Home", ""),
        ("Audio Input", "Hangátvitel"),
        ("Enhancements", "Fejlesztések"),
        ("Hardware Codec", "Hardware kodek"),
        ("Adaptive bitrate", "Adaptív bitráta"),
        ("ID Server", "Szerver azonosító/domain"),
        ("Relay Server", "Kiszolgáló szerver"),
        ("API Server", "API szerver"),
        ("invalid_http", "A címnek mindenképpen http(s)://-el kell kezdődnie."),
        ("Invalid IP", "A megadott IP cím helytelen."),
        ("Invalid format", "Érvénytelen formátum"),
        ("server_not_support", "Nem támogatott a szerver által"),
        ("Not available", "Nem elérhető"),
        ("Too frequent", "Túl gyakori"),
        ("Cancel", "Mégsem"),
        ("Skip", "Kihagyás"),
        ("Close", "Bezárás"),
        ("Retry", "Újra"),
        ("OK", "OK"),
        ("Password Required", "Jelszó megadása kötelező"),
        ("Please enter your password", "Kérem írja be a jelszavát"),
        ("Remember password", "Jelszó megjegyzése"),
        ("Wrong Password", "Hibás jelszó"),
        ("Do you want to enter again?", "Szeretne újra belépni?"),
        ("Connection Error", "Csatlakozási hiba"),
        ("Error", "Hiba"),
        ("Reset by the peer", "A kapcsolatot alaphelyzetbe állt"),
        ("Connecting...", "Csatlakozás..."),
        ("Connection in progress. Please wait.", "Csatlakozás folyamatban. Kérem várjon."),
        ("Please try 1 minute later", "Kérem próbálja meg 1 perc múlva"),
        ("Login Error", "Bejelentkezési hiba"),
        ("Successful", "Sikeres"),
        ("Connected, waiting for image...", "Csatlakozva, várakozás a kép adatokra..."),
        ("Name", "Név"),
        ("Type", "Típus"),
        ("Modified", "Módosított"),
        ("Size", "Méret"),
        ("Show Hidden Files", "Rejtett fájlok mutatása"),
        ("Receive", "Fogad"),
        ("Send", "Küld"),
        ("Refresh File", "Fájl frissítése"),
        ("Local", "Helyi"),
        ("Remote", "Távoli"),
        ("Remote Computer", "Távoli számítógép"),
        ("Local Computer", "Helyi számítógép"),
        ("Confirm Delete", "Törlés megerősítése"),
        ("Delete", "Törlés"),
        ("Properties", "Tulajdonságok"),
        ("Multi Select", "Többszörös kijelölés"),
        ("Select All", "Összes kijelölése"),
        ("Unselect All", "Kijelölések megszűntetése"),
        ("Empty Directory", "Üres könyvtár"),
        ("Not an empty directory", "Nem egy üres könyvtár"),
        ("Are you sure you want to delete this file?", "Biztosan törli ezt a fájlt?"),
        ("Are you sure you want to delete this empty directory?", "Biztosan törli ezt az üres könyvtárat?"),
        ("Are you sure you want to delete the file of this directory?", "Biztos benne, hogy törölni szeretné a könyvtár tartalmát?"),
        ("Do this for all conflicts", "Tegye ezt minden ütközéskor"),
        ("This is irreversible!", "Ez a folyamat visszafordíthatatlan!"),
        ("Deleting", "Törlés folyamatban"),
        ("files", "fájlok"),
        ("Waiting", "Várakozás"),
        ("Finished", "Befejezve"),
        ("Speed", "Sebesség"),
        ("Custom Image Quality", "Egyedi képminőség"),
        ("Privacy mode", "Inkognító mód"),
        ("Block user input", "Felhasználói bevitel letiltása"),
        ("Unblock user input", "Felhasználói bevitel engedélyezése"),
        ("Adjust Window", "Ablakméret beállítása"),
        ("Original", "Eredeti méret"),
        ("Shrink", "Kicsinyítés"),
        ("Stretch", "Nyújtás"),
        ("Scrollbar", "Görgetősáv"),
        ("ScrollAuto", "Automatikus görgetés"),
        ("Good image quality", "Eredetihez hű"),
        ("Balanced", "Kiegyensúlyozott"),
        ("Optimize reaction time", "Gyorsan reagáló"),
        ("Custom", "Egyedi"),
        ("Show remote cursor", "Távoli kurzor megjelenítése"),
        ("Show quality monitor", ""),
        ("Disable clipboard", "Közös vágólap kikapcsolása"),
        ("Lock after session end", "Távoli fiók zárolása a munkamenet végén"),
        ("Insert", ""),
        ("Insert Lock", "Távoli fiók zárolása"),
        ("Refresh", "Frissítés"),
        ("ID does not exist", "Az azonosító nem létezik"),
        ("Failed to connect to rendezvous server", "Nem sikerült csatlakozni a kiszolgáló szerverhez"),
        ("Please try later", "Kérjük, próbálja később"),
        ("Remote desktop is offline", "A távoli számítógép offline állapotban van"),
        ("Key mismatch", "Eltérés a kulcsokban"),
        ("Timeout", "Időtúllépés"),
        ("Failed to connect to relay server", "Nem sikerült csatlakozni a közvetítő szerverhez"),
        ("Failed to connect via rendezvous server", "Nem sikerült csatlakozni a kiszolgáló szerveren keresztül"),
        ("Failed to connect via relay server", "Nem sikerült csatlakozni a közvetítő szerveren keresztül"),
        ("Failed to make direct connection to remote desktop", "Nem sikerült közvetlen kapcsolatot létesíteni a távoli számítógéppel"),
        ("Set Password", "Jelszó Beállítása"),
        ("OS Password", "Operációs rendszer jelszavának beállítása"),
        ("install_tip", "Előfordul, hogy bizonyos esetekben hiba léphet fel a Portable verzió használata során. A megfelelő működés érdekében, kérem telepítse a RustDesk alkalmazást a számítógépre."),
        ("Click to upgrade", "Kattintson ide a frissítés telepítéséhez"),
        ("Click to download", "Kattintson ide a letöltéshez"),
        ("Click to update", "Kattintson ide a frissítés letöltéséhez"),
        ("Configure", "Beállítás"),
        ("config_acc", "A távoli vezérléshez a RustDesk-nek \"Kisegítő lehetőség\" engedélyre van szüksége"),
        ("config_screen", "A távoli vezérléshez szükséges a \"Képernyőfelvétel\" engedély megadása"),
        ("Installing ...", "Telepítés..."),
        ("Install", "Telepítés"),
        ("Installation", "Telepítés"),
        ("Installation Path", "Telepítési útvonal"),
        ("Create start menu shortcuts", "Start menü parancsikonok létrehozása"),
        ("Create desktop icon", "Ikon létrehozása az asztalon"),
        ("agreement_tip", "A telepítés folytatásával automatikusan elfogadásra kerül a licensz szerződés."),
        ("Accept and Install", "Elfogadás és telepítés"),
        ("End-user license agreement", "Felhasználói licensz szerződés"),
        ("Generating ...", "Létrehozás..."),
        ("Your installation is lower version.", "A telepített verzió alacsonyabb."),
        ("not_close_tcp_tip", "Ne zárja be ezt az ablakot miközben a tunnelt használja"),
        ("Listening ...", "Keresés..."),
        ("Remote Host", "Távoli kiszolgáló"),
        ("Remote Port", "Távoli port"),
        ("Action", "Indítás"),
        ("Add", "Hozzáadás"),
        ("Local Port", "Helyi port"),
        ("Local Address", "Helyi cím"),
        ("Change Local Port", "Helyi port megváltoztatása"),
        ("setup_server_tip", "Gyorsabb kapcsolat érdekében, hozzon létre saját szervert"),
        ("Too short, at least 6 characters.", "Túl rövid, legalább 6 karakter."),
        ("The confirmation is not identical.", "A megerősítés nem volt azonos"),
        ("Permissions", "Engedélyek"),
        ("Accept", "Elfogadás"),
        ("Dismiss", "Elutasítás"),
        ("Disconnect", "Kapcsolat bontása"),
        ("Allow using keyboard and mouse", "Billentyűzet és egér használatának engedélyezése"),
        ("Allow using clipboard", "Vágólap használatának engedélyezése"),
        ("Allow hearing sound", "Hang átvitelének engedélyezése"),
        ("Allow file copy and paste", "Fájlok másolásának és beillesztésének engedélyezése"),
        ("Connected", "Csatlakozva"),
        ("Direct and encrypted connection", "Közvetlen, és titkosított kapcsolat"),
        ("Relayed and encrypted connection", "Továbbított, és titkosított kapcsolat"),
        ("Direct and unencrypted connection", "Közvetlen, és nem titkosított kapcsolat"),
        ("Relayed and unencrypted connection", "Továbbított, és nem titkosított kapcsolat"),
        ("Enter Remote ID", "Távoli számítógép azonosítója"),
        ("Enter your password", "Írja be a jelszavát"),
        ("Logging in...", "A belépés folyamatban..."),
        ("Enable RDP session sharing", "RDP-munkamenet-megosztás engedélyezése"),
        ("Auto Login", "Automatikus bejelentkezés"),
        ("Enable Direct IP Access", "Közvetlen IP-elérés engedélyezése"),
        ("Rename", "Átnevezés"),
        ("Space", ""),
        ("Create Desktop Shortcut", "Asztali parancsikon létrehozása"),
        ("Change Path", "Elérési út módosítása"),
        ("Create Folder", "Mappa létrehozás"),
        ("Please enter the folder name", "Kérjük, adja meg a mappa nevét"),
        ("Fix it", "Javítás"),
        ("Warning", "Figyelmeztetés"),
        ("Login screen using Wayland is not supported", "Bejelentkezéskori Wayland használata nem támogatott"),
        ("Reboot required", "Újraindítás szükséges"),
        ("Unsupported display server", "Nem támogatott megjelenítő szerver"),
        ("x11 expected", "x11-re számítottt"),
        ("Port", "Port"),
        ("Settings", "Beállítások"),
        ("Username", "Felhasználónév"),
        ("Invalid port", "Érvénytelen port"),
        ("Closed manually by the peer", "A kapcsolatot a másik fél manuálisan bezárta"),
        ("Enable remote configuration modification", "Távoli konfiguráció módosítás engedélyezése"),
        ("Run without install", "Futtatás feltelepítés nélkül"),
        ("Connect via relay", ""),
        ("Always connect via relay", "Mindig közvetítőn keresztüli csatlakozás"),
        ("whitelist_tip", "Csak az engedélyezési listán szereplő címek csatlakozhatnak"),
        ("Login", "Belépés"),
        ("Verify", ""),
        ("Remember me", ""),
        ("Trust this device", ""),
        ("Verification code", ""),
        ("verification_tip", ""),
        ("Logout", "Kilépés"),
        ("Tags", "Tagok"),
        ("Search ID", "Azonosító keresése..."),
        ("whitelist_sep", "A címeket veszővel, pontosvesszővel, szóközzel, vagy új sorral válassza el"),
        ("Add ID", "Azonosító hozzáadása"),
        ("Add Tag", "Címke hozzáadása"),
        ("Unselect all tags", "A címkék kijelölésének megszüntetése"),
        ("Network error", "Hálózati hiba"),
        ("Username missed", "Üres felhasználónév"),
        ("Password missed", "Üres jelszó"),
        ("Wrong credentials", "Hibás felhasználónév vagy jelszó"),
        ("The verification code is incorrect or has expired", ""),
        ("Edit Tag", "Címke szerkesztése"),
        ("Unremember Password", "A jelszó megjegyzésének törlése"),
        ("Favorites", "Kedvencek"),
        ("Add to Favorites", "Hozzáadás a kedvencekhez"),
        ("Remove from Favorites", "Eltávolítás a kedvencekből"),
        ("Empty", "Üres"),
        ("Invalid folder name", "Helytelen mappa név"),
        ("Socks5 Proxy", "Socks5 Proxy"),
        ("Hostname", "Kiszolgáló név"),
        ("Discovered", "Felfedezett"),
        ("install_daemon_tip", "Az automatikus indításhoz szükséges a szolgáltatás telepítése"),
        ("Remote ID", "Távoli azonosító"),
        ("Paste", "Beillesztés"),
        ("Paste here?", "Beilleszti ide?"),
        ("Are you sure to close the connection?", "Biztos, hogy bezárja a kapcsolatot?"),
        ("Download new version", "Új verzó letöltése"),
        ("Touch mode", "Érintési mód bekapcsolása"),
        ("Mouse mode", "Egérhasználati mód bekapcsolása"),
        ("One-Finger Tap", "Egyujjas érintés"),
        ("Left Mouse", "Bal egér gomb"),
        ("One-Long Tap", "Hosszú érintés"),
        ("Two-Finger Tap", "Kétujjas érintés"),
        ("Right Mouse", "Jobb egér gomb"),
        ("One-Finger Move", "Egyujjas mozgatás"),
        ("Double Tap & Move", "Dupla érintés, és mozgatás"),
        ("Mouse Drag", "Mozgatás egérrel"),
        ("Three-Finger vertically", "Három ujj függőlegesen"),
        ("Mouse Wheel", "Egérgörgő"),
        ("Two-Finger Move", "Kátujjas mozgatás"),
        ("Canvas Move", "Nézet mozgatása"),
        ("Pinch to Zoom", "Kétujjas nagyítás"),
        ("Canvas Zoom", "Nézet nagyítása"),
        ("Reset canvas", "Nézet visszaállítása"),
        ("No permission of file transfer", "Nincs engedély a fájlátvitelre"),
        ("Note", "Megyjegyzés"),
        ("Connection", "Kapcsolat"),
        ("Share Screen", "Képernyőmegosztás"),
        ("Chat", "Chat"),
        ("Total", "Összes"),
        ("items", "elemek"),
        ("Selected", "Kijelölt"),
        ("Screen Capture", "Képernyőrögzítés"),
        ("Input Control", "Távoli vezérlés"),
        ("Audio Capture", "Hangrögzítés"),
        ("File Connection", "Fájlátvitel"),
        ("Screen Connection", "Képátvitel"),
        ("Do you accept?", "Elfogadja?"),
        ("Open System Setting", "Rendszerbeállítások megnyitása"),
        ("How to get Android input permission?", "Hogyan állíthatok be Android beviteli engedélyt?"),
        ("android_input_permission_tip1", "A távoli vezérléshez kérjük engedélyezze a \"Kisegítő lehetőség\" lehetőséget."),
        ("android_input_permission_tip2", "A következő rendszerbeállítások oldalon a letöltött alkalmazások menüponton belül, kapcsolja be a [RustDesk Input] szolgáltatást."),
        ("android_new_connection_tip", "Új kérés érkezett mely vezérelni szeretné az eszközét"),
        ("android_service_will_start_tip", "A \"Képernyőrögzítés\" bekapcsolásával automatikus elindul a szolgáltatás, lehetővé téve, hogy más eszközök csatlakozási kérelmet küldhessenek"),
        ("android_stop_service_tip", "A szolgáltatás leállítása automatikusan szétkapcsol minden létező kapcsolatot."),
        ("android_version_audio_tip", "A jelenlegi Android verzió nem támogatja a hangrögzítést, frissítsen legalább Android 10-re, vagy egy újabb verzióra."),
        ("android_start_service_tip", ""),
        ("android_permission_may_not_change_tip", ""),
        ("Account", "Fiók"),
        ("Overwrite", "Felülírás"),
        ("This file exists, skip or overwrite this file?", "Ez a fájl már létezik, kihagyja vagy felülírja ezt a fájlt?"),
        ("Quit", "Kilépés"),
        ("doc_mac_permission", "https://rustdesk.com/docs/hu/manual/mac/#enable-permissions"),
        ("Help", "Segítség"),
        ("Failed", "Sikertelen"),
        ("Succeeded", "Sikeres"),
        ("Someone turns on privacy mode, exit", "Valaki bekacsolta az inkognitó módot, lépjen ki"),
        ("Unsupported", "Nem támogatott"),
        ("Peer denied", "Elutasítva a távoli fél álltal"),
        ("Please install plugins", "Kérem telepítse a bővítményeket"),
        ("Peer exit", "A távoli fél kilépett"),
        ("Failed to turn off", "Nem sikerült kikapcsolni"),
        ("Turned off", "Kikapcsolva"),
        ("In privacy mode", "Belépés inkognitó módba"),
        ("Out privacy mode", "Kilépés inkognitó módból"),
        ("Language", "Nyelv"),
        ("Keep RustDesk background service", "RustDesk futtatása a háttérben"),
        ("Ignore Battery Optimizations", "Akkumulátorkímélő figyelmen kívűl hagyása"),
        ("android_open_battery_optimizations_tip", "Ha le szeretné tiltani ezt a funkciót, lépjen a RustDesk alkalmazás beállítási oldalára, keresse meg az [Akkumulátorkímélő] lehetőséget és válassza a nincs korlátozás lehetőséget."),
        ("Start on Boot", ""),
        ("Start the screen sharing service on boot, requires special permissions", ""),
        ("Connection not allowed", "A csatlakozás nem engedélyezett"),
        ("Legacy mode", ""),
        ("Map mode", ""),
        ("Translate mode", "Fordító mód"),
        ("Use permanent password", "Állandó jelszó használata"),
        ("Use both passwords", "Mindkét jelszó használata"),
        ("Set permanent password", "Állandó jelszó beállítása"),
        ("Enable Remote Restart", "Távoli újraindítás engedélyezése"),
        ("Allow remote restart", "Távoli újraindítás engedélyezése"),
        ("Restart Remote Device", "Távoli eszköz újraindítása"),
        ("Are you sure you want to restart", "Biztos szeretné újraindítani?"),
        ("Restarting Remote Device", "Távoli eszköz újraindítása..."),
        ("remote_restarting_tip", "A távoli eszköz újraindul, zárja be ezt az üzenetet, csatlakozzon újra, állandó jelszavával"),
        ("Copied", "Másolva"),
        ("Exit Fullscreen", "Kilépés teljes képernyős módból"),
        ("Fullscreen", "Teljes képernyő"),
        ("Mobile Actions", "mobil műveletek"),
        ("Select Monitor", "Válasszon képernyőt"),
        ("Control Actions", "Irányítási műveletek"),
        ("Display Settings", "Megjelenítési beállítások"),
        ("Ratio", "Arány"),
        ("Image Quality", "Képminőség"),
        ("Scroll Style", "Görgetési stílus"),
        ("Show Toolbar", ""),
        ("Hide Toolbar", ""),
        ("Direct Connection", "Közvetlen kapcsolat"),
        ("Relay Connection", "Közvetett csatlakozás"),
        ("Secure Connection", "Biztonságos kapcsolat"),
        ("Insecure Connection", "Nem biztonságos kapcsolat"),
        ("Scale original", "Eredeti méretarány"),
        ("Scale adaptive", "Adaptív méretarány"),
        ("General", "Általános"),
        ("Security", "Biztonság"),
        ("Theme", "Téma"),
        ("Dark Theme", "Sötét téma"),
        ("Light Theme", ""),
        ("Dark", "Sötét"),
        ("Light", "Világos"),
        ("Follow System", ""),
        ("Enable hardware codec", "Hardveres kodek engedélyezése"),
        ("Unlock Security Settings", "Biztonsági beállítások feloldása"),
        ("Enable Audio", "Hang engedélyezése"),
        ("Unlock Network Settings", "Hálózati beállítások feloldása"),
        ("Server", "Szerver"),
        ("Direct IP Access", "Közvetlen IP hozzáférés"),
        ("Proxy", "Proxy"),
        ("Apply", "Alkalmaz"),
        ("Disconnect all devices?", "Leválasztja az összes eszközt?"),
        ("Clear", "Tisztítás"),
        ("Audio Input Device", "Audio bemeneti eszköz"),
        ("Use IP Whitelisting", "Engedélyezési lista használata"),
        ("Network", "Hálózat"),
        ("Enable RDP", "RDP engedélyezése"),
        ("Pin Toolbar", ""),
        ("Unpin Toolbar", ""),
        ("Recording", "Felvétel"),
        ("Directory", "Könyvtár"),
        ("Automatically record incoming sessions", "A bejövő munkamenetek automatikus rögzítése"),
        ("Change", "Változtatás"),
        ("Start session recording", "Munkamenet rögzítés indítása"),
        ("Stop session recording", "Munkamenet rögzítés leállítása"),
        ("Enable Recording Session", "Munkamenet rögzítés engedélyezése"),
        ("Allow recording session", "Munkamenet rögzítés engedélyezése"),
        ("Enable LAN Discovery", "Felfedezés enegedélyezése"),
        ("Deny LAN Discovery", "Felfedezés tiltása"),
        ("Write a message", "Üzenet írása"),
        ("Prompt", ""),
        ("Please wait for confirmation of UAC...", ""),
        ("elevated_foreground_window_tip", ""),
        ("Disconnected", "Szétkapcsolva"),
        ("Other", "Egyéb"),
        ("Confirm before closing multiple tabs", "Biztos, hogy bezárja az összes lapot?"),
        ("Keyboard Settings", "Billentyűzet beállítások"),
        ("Full Access", "Teljes hozzáférés"),
        ("Screen Share", "Képernyőmegosztás"),
        ("Wayland requires Ubuntu 21.04 or higher version.", "A Waylandhoz Ubuntu 21.04 vagy újabb verzió szükséges."),
        ("Wayland requires higher version of linux distro. Please try X11 desktop or change your OS.", "A Wayland a Linux disztró magasabb verzióját igényli. Próbálja ki az X11 desktopot, vagy változtassa meg az operációs rendszert."),
        ("JumpLink", "Hiperhivatkozás"),
        ("Please Select the screen to be shared(Operate on the peer side).", "Kérjük, válassza ki a megosztani kívánt képernyőt."),
        ("Show RustDesk", "A RustDesk megjelenítése"),
        ("This PC", "Ez a számítógép"),
        ("or", "vagy"),
        ("Continue with", "Folytatás a következővel"),
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
        ("Long press to select tabs", ""),
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
        ("Reverse mouse wheel", ""),
        ("{} sessions", ""),
        ("scam_title", ""),
        ("scam_text1", ""),
        ("scam_text2", ""),
        ("Don't show again", ""),
        ("I Agree", ""),
        ("Decline", ""),
        ("Timeout in minutes", ""),
        ("auto_disconnect_option_tip", ""),
        ("Connection failed due to inactivity", ""),
        ("Check for software update on startup", ""),
    ].iter().cloned().collect();
}
