lazy_static::lazy_static! {
pub static ref T: std::collections::HashMap<&'static str, &'static str> =
    [
        ("Status", "Status"),
        ("Your Desktop", "Desktop Anda"),
        ("desk_tip", "Desktop Anda dapat diakses dengan ID dan kata sandi ini."),
        ("Password", "Password"),
        ("Ready", "Siap"),
        ("Established", "Didirikan"),
        ("connecting_status", "Menghubungkan ke jaringan RustDesk..."),
        ("Enable Service", "Aktifkan Layanan"),
        ("Start Service", "Mulai Layanan"),
        ("Service is running", "Layanan berjalan"),
        ("Service is not running", "Layanan tidak berjalan"),
        ("not_ready_status", "Belum siap. Silakan periksa koneksi Anda"),
        ("Control Remote Desktop", "Kontrol Remote Desktop"),
        ("Transfer File", "File Transfer"),
        ("Connect", "Menghubung"),
        ("Recent Sessions", "Sesi Terkini"),
        ("Address Book", "Buku Alamat"),
        ("Confirmation", "Konfirmasi"),
        ("TCP Tunneling", "TCP Tunneling"),
        ("Remove", "Hapus"),
        ("Refresh random password", "Segarkan kata sandi acak"),
        ("Set your own password", "Tetapkan kata sandi Anda sendiri"),
        ("Enable Keyboard/Mouse", "Aktifkan Keyboard/Mouse"),
        ("Enable Clipboard", "Aktifkan Papan Klip"),
        ("Enable File Transfer", "Aktifkan Transfer File"),
        ("Enable TCP Tunneling", "Aktifkan TCP Tunneling"),
        ("IP Whitelisting", "Daftar Putih IP"),
        ("ID/Relay Server", "ID/Relay Server"),
        ("Import Server Config", "Impor Konfigurasi Server"),
        ("Export Server Config", ""),
        ("Import server configuration successfully", "Impor konfigurasi server berhasil"),
        ("Export server configuration successfully", ""),
        ("Invalid server configuration", "Konfigurasi server tidak valid"),
        ("Clipboard is empty", "Papan klip kosong"),
        ("Stop service", "Hentikan Layanan"),
        ("Change ID", "Ubah ID"),
        ("Website", "Website"),
        ("About", "Tentang"),
        ("Mute", "Bisukan"),
        ("Audio Input", "Masukkan Audio"),
        ("Enhancements", "Peningkatan"),
        ("Hardware Codec", "Codec Perangkat Keras"),
        ("Adaptive Bitrate", "Kecepatan Bitrate Adaptif"),
        ("ID Server", "Server ID"),
        ("Relay Server", "Server Relay"),
        ("API Server", "API Server"),
        ("invalid_http", "harus dimulai dengan http:// atau https://"),
        ("Invalid IP", "IP tidak valid"),
        ("id_change_tip", "Hanya karakter a-z, A-Z, 0-9 dan _ (underscore) yang diperbolehkan. Huruf pertama harus a-z, A-Z. Panjang antara 6 dan 16."),
        ("Invalid format", "Format tidak valid"),
        ("server_not_support", "Belum didukung oleh server"),
        ("Not available", "Tidak tersedia"),
        ("Too frequent", "Terlalu sering"),
        ("Cancel", "Batal"),
        ("Skip", "Lanjutkan"),
        ("Close", "Tutup"),
        ("Retry", "Ulangi"),
        ("OK", "OK"),
        ("Password Required", "Password dibutukan"),
        ("Please enter your password", "Silahkan masukkan password anda"),
        ("Remember password", "Ingat Password"),
        ("Wrong Password", "Password Salah"),
        ("Do you want to enter again?", "Apakah anda ingin masuk lagi?"),
        ("Connection Error", "Kesalahan koneksi"),
        ("Error", "Kesalahan"),
        ("Reset by the peer", "Setel ulang oleh rekan"),
        ("Connecting...", "Hubungkan..."),
        ("Connection in progress. Please wait.", "Koneksi sedang berlangsung. Mohon tunggu."),
        ("Please try 1 minute later", "Silahkan coba 1 menit lagi"),
        ("Login Error", "Kesalahan Login"),
        ("Successful", "Berhasil"),
        ("Connected, waiting for image...", "Terhubung, menunggu gambar..."),
        ("Name", "Nama"),
        ("Type", "Tipe"),
        ("Modified", "Diperbarui"),
        ("Size", "Ukuran"),
        ("Show Hidden Files", "Tampilkan File Tersembunyi"),
        ("Receive", "Menerima"),
        ("Send", "Kirim"),
        ("Refresh File", "Segarkan File"),
        ("Local", "Lokal"),
        ("Remote", "Remote"),
        ("Remote Computer", "Remote Komputer"),
        ("Local Computer", "Lokal Komputer"),
        ("Confirm Delete", "Konfirmasi Hapus"),
        ("Delete", "Hapus"),
        ("Properties", "Properti"),
        ("Multi Select", "Pilih Beberapa"),
        ("Select All", "Pilih Semua"),
        ("Unselect All", "Batalkan Pilihan Semua"),
        ("Empty Directory", "Folder Kosong"),
        ("Not an empty directory", "Folder tidak kosong"),
        ("Are you sure you want to delete this file?", "Apakah anda yakin untuk menghapus file ini?"),
        ("Are you sure you want to delete this empty directory?", "Apakah anda yakin untuk menghapus folder ini?"),
        ("Are you sure you want to delete the file of this directory?", "Apakah anda yakin untuk menghapus file dan folder ini?"),
        ("Do this for all conflicts", "Lakukan untuk semua konflik"),
        ("This is irreversible!", "Ini tidak dapat diubah!"),
        ("Deleting", "Menghapus"),
        ("files", "file"),
        ("Waiting", "Menunggu"),
        ("Finished", "Selesai"),
        ("Speed", "Kecepatan"),
        ("Custom Image Quality", "Sesuaikan Kualitas Gambar"),
        ("Privacy mode", "Mode Privasi"),
        ("Block user input", "Blokir masukan pengguna"),
        ("Unblock user input", "Jangan blokir masukan pengguna"),
        ("Adjust Window", "Sesuaikan Jendela"),
        ("Original", "Original"),
        ("Shrink", "Susutkan"),
        ("Stretch", "Regangkan"),
        ("Scrollbar", "Scroll bar"),
        ("ScrollAuto", "Gulir Otomatis"),
        ("Good image quality", "Kualitas Gambar Baik"),
        ("Balanced", "Seimbang"),
        ("Optimize reaction time", "Optimalkan waktu reaksi"),
        ("Custom", "Kustom"),
        ("Show remote cursor", "Tampilkan remote kursor"),
        ("Show quality monitor", "Tampilkan kualitas monitor"),
        ("Disable clipboard", "Matikan papan klip"),
        ("Lock after session end", "Kunci setelah sesi berakhir"),
        ("Insert", "Menyisipkan"),
        ("Insert Lock", "Masukkan Kunci"),
        ("Refresh", "Segarkan"),
        ("ID does not exist", "ID tidak ada"),
        ("Failed to connect to rendezvous server", "Gagal menghubungkan ke rendezvous server"),
        ("Please try later", "Silahkan coba lagi nanti"),
        ("Remote desktop is offline", "Remote desktop offline"),
        ("Key mismatch", "Ketidakcocokan kunci"),
        ("Timeout", "Waktu habis"),
        ("Failed to connect to relay server", "Gagal terkoneksi ke relay server"),
        ("Failed to connect via rendezvous server", "Gagal terkoneksi via rendezvous server"),
        ("Failed to connect via relay server", "Gagal terkoneksi via relay server"),
        ("Failed to make direct connection to remote desktop", "Gagal membuat koneksi langsung ke desktop jarak jauh"),
        ("Set Password", "Tetapkan Password"),
        ("OS Password", "Kata Sandi OS"),
        ("install_tip", "Karena UAC, RustDesk tidak dapat bekerja dengan baik sebagai sisi remote dalam beberapa kasus. Untuk menghindari UAC, silakan klik tombol di bawah ini untuk menginstal RustDesk ke sistem."),
        ("Click to upgrade", "Klik untuk upgrade"),
        ("Click to download", "Kli untuk download"),
        ("Click to update", "Klik untuk update"),
        ("Configure", "Konfigurasi"),
        ("config_acc", "Untuk mengontrol Desktop Anda dari jarak jauh, Anda perlu memberikan izin \"Aksesibilitas\" RustDesk."),
        ("config_screen", "Untuk mengakses Desktop Anda dari jarak jauh, Anda perlu memberikan izin \"Perekaman Layar\" RustDesk."),
        ("Installing ...", "Menginstall"),
        ("Install", "Instal"),
        ("Installation", "Instalasi"),
        ("Installation Path", "Jalur Instalasi"),
        ("Create start menu shortcuts", "Buat pintasan start menu"),
        ("Create desktop icon", "Buat icon desktop"),
        ("agreement_tip", "Dengan memulai instalasi, Anda menerima perjanjian lisensi."),
        ("Accept and Install", "Terima dan Install"),
        ("End-user license agreement", "Perjanjian lisensi pengguna akhir"),
        ("Generating ...", "Menghasilkan..."),
        ("Your installation is lower version.", "Instalasi Anda adalah versi yang lebih rendah."),
        ("not_close_tcp_tip", "Jangan tutup jendela ini saat menggunakan tunnel"),
        ("Listening ...", "Mendengarkan..."),
        ("Remote Host", "Remote Host"),
        ("Remote Port", "Remote Port"),
        ("Action", "Aksi"),
        ("Add", "Tambah"),
        ("Local Port", "Port Lokal"),
        ("Local Address", "Alamat lokal"),
        ("Change Local Port", "Ubah Port Lokal"),
        ("setup_server_tip", "Untuk koneksi yang lebih cepat, silakan atur server Anda sendiri"),
        ("Too short, at least 6 characters.", "Terlalu pendek, setidaknya 6 karekter."),
        ("The confirmation is not identical.", "Konfirmasi tidak identik."),
        ("Permissions", "Izin"),
        ("Accept", "Terima"),
        ("Dismiss", "Hentikan"),
        ("Disconnect", "Terputus"),
        ("Allow using keyboard and mouse", "Izinkan menggunakan keyboard dan mouse"),
        ("Allow using clipboard", "Izinkan menggunakan papan klip"),
        ("Allow hearing sound", "Izinkan mendengarkan suara"),
        ("Allow file copy and paste", "Izinkan penyalinan dan tempel file"),
        ("Connected", "Terkoneksi"),
        ("Direct and encrypted connection", "Koneksi langsung dan terenkripsi"),
        ("Relayed and encrypted connection", "Koneksi relai dan terenkripsi"),
        ("Direct and unencrypted connection", "Koneksi langsung dan tidak terenkripsi"),
        ("Relayed and unencrypted connection", "Koneksi relai dan tidak terenkripsi"),
        ("Enter Remote ID", "Masukkan Remote ID"),
        ("Enter your password", "Masukkan password anda"),
        ("Logging in...", "Masuk..."),
        ("Enable RDP session sharing", "Aktifkan berbagi sesi RDP"),
        ("Auto Login", "Auto Login (Hanya valid jika Anda menyetel \"Kunci setelah sesi berakhir\")"),
        ("Enable Direct IP Access", "Aktifkan Akses IP Langsung"),
        ("Rename", "Ubah nama"),
        ("Space", "Spasi"),
        ("Create Desktop Shortcut", "Buat Pintasan Desktop"),
        ("Change Path", "Ubah Jalur"),
        ("Create Folder", "Buat Folder"),
        ("Please enter the folder name", "Silahkan masukkan nama folder"),
        ("Fix it", "Memperbaiki"),
        ("Warning", "Peringatan"),
        ("Login screen using Wayland is not supported", "Layar masuk menggunakan Wayland tidak didukung"),
        ("Reboot required", "Diperlukan boot ulang"),
        ("Unsupported display server ", "Server tampilan tidak didukung "),
        ("x11 expected", "x11 diharapkan"),
        ("Port", "Port"),
        ("Settings", "Pengaturan"),
        ("Username", "Username"),
        ("Invalid port", "Kesalahan port"),
        ("Closed manually by the peer", "Ditutup secara manual oleh peer"),
        ("Enable remote configuration modification", "Aktifkan modifikasi konfigurasi jarak jauh"),
        ("Run without install", "Jalankan tanpa menginstal"),
        ("Always connected via relay", "Selalu terhubung melalui relai"),
        ("Always connect via relay", "Selalu terhubung melalui relai"),
        ("whitelist_tip", "Hanya whitelisted IP yang dapat mengakses saya"),
        ("Login", "Masuk"),
        ("Logout", "Keluar"),
        ("Tags", "Tag"),
        ("Search ID", "Cari ID"),
        ("Current Wayland display server is not supported", "Server tampilan Wayland saat ini tidak didukung"),
        ("whitelist_sep", "Dipisahkan dengan koma, titik koma, spasi, atau baris baru"),
        ("Add ID", "Tambah ID"),
        ("Add Tag", "Tambah Tag"),
        ("Unselect all tags", "Batalkan pilihan semua tag"),
        ("Network error", "Kesalahan Jaringan"),
        ("Username missed", "Username tidak sesuai"),
        ("Password missed", "Kata sandi tidak sesuai"),
        ("Wrong credentials", "Username atau password salah"),
        ("Edit Tag", "Ubah Tag"),
        ("Unremember Password", "Lupa Kata Sandi"),
        ("Favorites", "Favorit"),
        ("Add to Favorites", "Tambah ke Favorit"),
        ("Remove from Favorites", "Hapus dari favorit"),
        ("Empty", "Kosong"),
        ("Invalid folder name", "Nama folder tidak valid"),
        ("Socks5 Proxy", "Socks5 Proxy"),
        ("Hostname", "Hostname"),
        ("Discovered", "Telah ditemukan"),
        ("install_daemon_tip", "Untuk memulai saat boot, Anda perlu menginstal system service."),
        ("Remote ID", "Remote ID"),
        ("Paste", "Tempel"),
        ("Paste here?", "Tempel disini?"),
        ("Are you sure to close the connection?", "Apakah anda yakin akan menutup koneksi?"),
        ("Download new version", "Untuk versi baru"),
        ("Touch mode", "Mode Sentuh"),
        ("Mouse mode", "Mode Mouse"),
        ("One-Finger Tap", "Ketuk Satu Jari"),
        ("Left Mouse", "Mouse Kiri"),
        ("One-Long Tap", "Ketuk Satu Panjang"),
        ("Two-Finger Tap", "Ketuk Dua Jari"),
        ("Right Mouse", "Mouse Kanan"),
        ("One-Finger Move", "Gerakan Satu Jari"),
        ("Double Tap & Move", "Ketuk Dua Kali & Pindah"),
        ("Mouse Drag", "Geser Mouse"),
        ("Three-Finger vertically", "Tiga Jari secara vertikal"),
        ("Mouse Wheel", "Roda mouse"),
        ("Two-Finger Move", "Gerakan Dua Jari"),
        ("Canvas Move", "Gerakan Kanvas"),
        ("Pinch to Zoom", "Cubit untuk Memperbesar"),
        ("Canvas Zoom", "Perbesar Canvas"),
        ("Reset canvas", "Setel Ulang Canvas"),
        ("No permission of file transfer", "Tidak ada izin untuk mengirim file"),
        ("Note", "Catatan"),
        ("Connection", "Koneksi"),
        ("Share Screen", "Bagikan Layar"),
        ("CLOSE", "TUTUP"),
        ("OPEN", "BUKA"),
        ("Chat", "Obrolan"),
        ("Total", "Total"),
        ("items", "item"),
        ("Selected", "Dipilih"),
        ("Screen Capture", "Rekam Layar"),
        ("Input Control", "kontrol input"),
        ("Audio Capture", "Rekam Suara"),
        ("File Connection", "Koneksi File"),
        ("Screen Connection", "koneksi layar"),
        ("Do you accept?", "Apakah diperbolehkan?"),
        ("Open System Setting", "Buka Pengaturan Sistem"),
        ("How to get Android input permission?", ""),
        ("android_input_permission_tip1", "Agar perangkat jarak jauh dapat mengontrol perangkat Android Anda melalui mouse atau sentuhan, Anda harus mengizinkan RustDesk untuk menggunakan layanan \"Aksesibilitas\"."),
        ("android_input_permission_tip2", "Silakan buka halaman pengaturan sistem berikutnya, temukan dan masuk ke [Layanan Terinstal], aktifkan layanan [Input RustDesk]."),
        ("android_new_connection_tip", "Permintaan kontrol baru telah diterima, yang ingin mengontrol perangkat Anda saat ini."),
        ("android_service_will_start_tip", "Mengaktifkan \"Tangkapan Layar\" akan memulai layanan secara otomatis, memungkinkan perangkat lain untuk meminta sambungan ke perangkat Anda."),
        ("android_stop_service_tip", "Menutup layanan akan secara otomatis menutup semua koneksi yang dibuat."),
        ("android_version_audio_tip", "Versi Android saat ini tidak mendukung pengambilan audio, harap tingkatkan ke Android 10 atau lebih tinggi."),
        ("android_start_service_tip", "Ketuk izin [Mulai Layanan] atau BUKA [Tangkapan Layar] untuk memulai layanan berbagi layar."),
        ("Account", "Akun"),
        ("Overwrite", "Timpa"),
        ("This file exists, skip or overwrite this file?", "File ini sudah ada, lewati atau timpa file ini?"),
        ("Quit", "Keluar"),
        ("doc_mac_permission", "https://rustdesk.com/docs/en/manual/mac/#enable-permissions"),
        ("Help", "Bantuan"),
        ("Failed", "Gagal"),
        ("Succeeded", "Berhasil"),
        ("Someone turns on privacy mode, exit", "Seseorang mengaktifkan mode privasi, keluar"),
        ("Unsupported", "Tidak didukung"),
        ("Peer denied", "Rekan ditolak"),
        ("Please install plugins", "Silakan instal plugin"),
        ("Peer exit", "keluar rekan"),
        ("Failed to turn off", "Gagal mematikan"),
        ("Turned off", "Matikan"),
        ("In privacy mode", "Dalam mode privasi"),
        ("Out privacy mode", "Keluar dari mode privasi"),
        ("Language", "Bahasa"),
        ("Keep RustDesk background service", "Pertahankan RustDesk berjalan pada background service"),
        ("Ignore Battery Optimizations", "Abaikan Pengoptimalan Baterai"),
        ("android_open_battery_optimizations_tip", ""),
        ("Connection not allowed", "Koneksi tidak dijinkan"),
        ("Legacy mode", "Mode lama"),
        ("Map mode", "Mode peta"),
        ("Translate mode", "Mode terjemahan"),
        ("Use temporary password", "Gunakan kata sandi sementara"),
        ("Use permanent password", "Gunakan kata sandi permanaen"),
        ("Use both passwords", "Gunakan kedua kata sandi "),
        ("Set permanent password", "Setel kata sandi permanen"),
        ("Set temporary password length", "Setel panjang kata sandi sementara"),
        ("Enable Remote Restart", "Aktifkan Restart Jarak Jauh"),
        ("Allow remote restart", "Ijinkan Restart Jarak Jauh"),
        ("Restart Remote Device", "Restart Perangkat Jarak Jauh"),
        ("Are you sure you want to restart", "Apakah Anda yakin untuk memulai ulang"),
        ("Restarting Remote Device", "Memulai Ulang Perangkat Jarak Jauh"),
        ("remote_restarting_tip", ""),
        ("Copied", "Disalin"),
        ("Exit Fullscreen", "Keluar dari Layar Penuh"),
        ("Fullscreen", "Layar penuh"),
        ("Mobile Actions", "Tindakan Seluler"),
        ("Select Monitor", "Pilih Monitor"),
        ("Control Actions", "Tindakan Kontrol"),
        ("Display Settings", "Pengaturan tampilan"),
        ("Ratio", "Perbandingan"),
        ("Image Quality", "Kualitas gambar"),
        ("Scroll Style", "Gaya Gulir"),
        ("Show Menubar", "Tampilkan bilah menu"),
        ("Hide Menubar", "sembunyikan bilah menu"),
        ("Direct Connection", "Koneksi langsung"),
        ("Relay Connection", "Koneksi Relay"),
        ("Secure Connection", "Koneksi aman"),
        ("Insecure Connection", "Koneksi Tidak Aman"),
        ("Scale original", "Skala asli"),
        ("Scale adaptive", "Skala adaptif"),
        ("General", "Umum"),
        ("Security", "Keamanan"),
        ("Account", "Akun"),
        ("Theme", "Tema"),
        ("Dark Theme", "Tema gelap"),
        ("Dark", "Gelap"),
        ("Light", "Terang"),
        ("Follow System", "Ikuti sistem"),
        ("Enable hardware codec", "Aktifkan codec perangkat keras"),
        ("Unlock Security Settings", "Buka Kunci Pengaturan Keamanan"),
        ("Enable Audio", "Aktifkan Audio"),
        ("Temporary Password Length", "Panjang Kata Sandi Sementara"),
        ("Unlock Network Settings", "Buka Kunci Pengaturan Jaringan"),
        ("Server", "Server"),
        ("Direct IP Access", "Direct IP Access"),
        ("Proxy", "Proxy"),
        ("Port", "Port"),
        ("Apply", "Terapkan"),
        ("Disconnect all devices?", "Putuskan sambungan semua perangkat?"),
        ("Clear", ""),
        ("Audio Input Device", ""),
        ("Deny remote access", "Tolak akses jarak jauh"),
        ("Use IP Whitelisting", "Gunakan Daftar Putih IP"),
        ("Network", "Jaringan"),
        ("Enable RDP", "Aktifkan RDP"),
        ("Pin menubar", "Pin menubar"),
        ("Unpin menubar", "Unpin menubar"),
        ("Recording", "Rekaman"),
        ("Directory", "Direktori"),
        ("Automatically record incoming sessions", "Secara otomatis merekam sesi masuk"),
        ("Change", "Mengubah"),
        ("Start session recording", "Mulai sesi perekaman"),
        ("Stop session recording", "Hentikan sesi perekaman"),
        ("Enable Recording Session", "Aktifkan Sesi Perekaman"),
        ("Allow recording session", "Izinkan sesi perekaman"),
        ("Enable LAN Discovery", "Aktifkan Penemuan LAN"),
        ("Deny LAN Discovery", "Tolak Penemuan LAN"),
        ("Write a message", "Menulis pesan"),
        ("Prompt", ""),
        ("elevation_prompt", ""),
        ("uac_warning", ""),
        ("elevated_foreground_window_warning", ""),
        ("Disconnected", "Terputus"),
        ("Other", "Lainnya"),
        ("Confirm before closing multiple tabs", "Konfirmasi sebelum menutup banyak tab"),
        ("Keyboard Settings", "Pengaturan Papan Ketik"),
        ("Custom", "Kustom"),
        ("Full Access", "Akses penuh"),
        ("Screen Share", "Berbagi Layar"),
        ("Wayland requires Ubuntu 21.04 or higher version.", "Wayland membutuhkan Ubuntu 21.04 atau versi yang lebih tinggi."),
        ("Wayland requires higher version of linux distro. Please try X11 desktop or change your OS.", "Wayland membutuhkan versi distro linux yang lebih tinggi. Silakan coba desktop X11 atau ubah OS Anda."),
        ("JumpLink", "View"),
        ("Please Select the screen to be shared(Operate on the peer side).", "Silakan Pilih layar yang akan dibagikan (Operasi di sisi rekan)."),
        ("Switch Sides", ""),
        ("Please confirm if you want to share your desktop ?", ""),
    ].iter().cloned().collect();
}
