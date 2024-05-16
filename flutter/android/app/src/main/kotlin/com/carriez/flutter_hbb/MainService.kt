package com.carriez.flutter_hbb

/**
 * Capture screen,get video and audio,send to rust.
 * Dispatch notifications
 *
 * Inspired by [droidVNC-NG] https://github.com/bk138/droidVNC-NG
 */

import android.annotation.SuppressLint
import android.app.Notification
import android.app.Notification.FOREGROUND_SERVICE_IMMEDIATE
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.PendingIntent.FLAG_IMMUTABLE
import android.app.PendingIntent.FLAG_UPDATE_CURRENT
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.Color
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR
import android.hardware.display.VirtualDisplay
import android.media.ImageReader
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.PowerManager
import android.util.Log
import android.view.Surface
import android.view.Surface.FRAME_RATE_COMPATIBILITY_DEFAULT
import androidx.annotation.Keep
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import ffi.FFI
import io.flutter.embedding.android.FlutterActivity
import org.json.JSONException
import org.json.JSONObject
import java.util.concurrent.Executors


const val DEFAULT_NOTIFY_TITLE = "RustDesk"
const val DEFAULT_NOTIFY_TEXT = "Service is running"
const val DEFAULT_NOTIFY_ID = 11
const val NOTIFY_ID_OFFSET = 100

const val MIME_TYPE = MediaFormat.MIMETYPE_VIDEO_VP9

// video const

const val VIDEO_KEY_BIT_RATE = 1024_000
const val VIDEO_KEY_FRAME_RATE = 30

class MainService : Service() {

    @Keep
    @RequiresApi(Build.VERSION_CODES.N)
    fun rustPointerInput(kind: String, mask: Int, x: Int, y: Int) {

    }

    @Keep
    @RequiresApi(Build.VERSION_CODES.N)
    fun rustKeyEventInput(input: ByteArray) {
    }


    @Keep
    fun rustGetByName(name: String): String {
        return when (name) {
            "screen_size" -> {
                JSONObject().apply {
                    put("width",SCREEN_INFO.width)
                    put("height",SCREEN_INFO.height)
                    put("scale",SCREEN_INFO.scale)
                }.toString()
            }
            else -> ""
        }
    }

    @Keep
    fun rustSetByName(name: String, arg1: String, arg2: String) {
        when (name) {
            "add_connection" -> {
                startMediaProject();
            }
            "update_voice_call_state" -> {

            }
            "stop_capture" -> {
                MediaProjectionService.instance?.stopCapture()
            }
            else -> {
            }
        }
    }

    private fun translate(input: String): String {
        Log.d(logTag, "translate:$LOCAL_NAME")
        return FFI.translateLocale(LOCAL_NAME, input)
    }

    companion object {
        public var instance: MainService? = null
        private var _isReady = false // media permission ready status
        public var _isStart = false // screen capture start status
        public var _isAudioStart = false // audio capture start status
        val isReady: Boolean
            get() = _isReady
        val isStart: Boolean
            get() = _isStart

       fun setIsStart(value: Boolean) {
            _isStart = value
        }

        val isAudioStart: Boolean
            get() = _isAudioStart
    }

    private val logTag = "LOG_SERVICE"


    override fun onCreate() {
        super.onCreate()
        instance = this
        FFI.init(this)

        // keep the config dir same with flutter
        val prefs = applicationContext.getSharedPreferences(KEY_SHARED_PREFERENCES, FlutterActivity.MODE_PRIVATE)
        val configPath = prefs.getString(KEY_APP_DIR_CONFIG_PATH, "") ?: ""
        FFI.startServer(configPath, "")

        Log.d(logTag, "call createForegroundNotification in onCreate")
//        initNotification()
//        createForegroundNotification()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            /*
                Create notification channel
             */
            val serviceChannel = NotificationChannel(
                packageName,
                "Foreground Service Channel",
                NotificationManager.IMPORTANCE_DEFAULT
            )
            val manager = getSystemService(
                NotificationManager::class.java
            )
            manager.createNotificationChannel(serviceChannel)
            startForeground(DEFAULT_NOTIFY_ID, getNotification(null, true))

        }
    }

    override fun onDestroy() {
        super.onDestroy()
        instance = null
    }

     override fun onBind(intent: Intent?): IBinder? {
         return null
     }

    private var mResultCode = 0
    private var mResultData: Intent? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.d(logTag, "onStartCommand" + intent?.action)
        Log.d("whichService", "this service: ${Thread.currentThread()}")
        super.onStartCommand(intent, flags, startId)
        if (intent?.action == ACT_INIT_MEDIA_PROJECTION_AND_SERVICE) {
            if (intent.getBooleanExtra(EXT_INIT_FROM_BOOT, false)) {
                FFI.startService()
            }
            // This initiates a prompt dialog for the user to confirm screen projection.
            val mediaProjectionRequestIntent = Intent(
                this,
                MediaProjectionRequestActivity::class.java
            )
            mediaProjectionRequestIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            startActivity(mediaProjectionRequestIntent)

        } else if (intent?.action == ACTION_HANDLE_MEDIA_PROJECTION_RESULT) {
            Log.d(logTag, "service starting: ${startId}:${Thread.currentThread()}")
            val mediaProjectionManager =
                getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager

            // Step 4 (optional): coming back from capturing permission check, now starting capturing machinery
            mResultCode = intent.getIntExtra(EXTRA_MEDIA_PROJECTION_RESULT_CODE, 0)
            mResultData =
                intent.getParcelableExtra<Intent>(EXTRA_MEDIA_PROJECTION_RESULT_DATA)

            if (mResultData != null) {
                _isReady = true
            }
        }
        return START_NOT_STICKY // don't use sticky (auto restart), the new service (from auto restart) will lose control
    }



    fun destroy() {
        Log.d(logTag, "destroy service")
        _isReady = false
        _isAudioStart = false
        stopForeground(true)
        stopSelf()
    }

    fun startMediaProject() {
        val intent = Intent(this, MediaProjectionService::class.java)
        intent.putExtra(EXTRA_MEDIA_PROJECTION_RESULT_CODE, mResultCode)
        intent.putExtra(EXTRA_MEDIA_PROJECTION_RESULT_DATA, mResultData)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
    }

    fun checkMediaPermission(): Boolean {
        Handler(Looper.getMainLooper()).post {
            MainActivity.flutterMethodChannel?.invokeMethod(
                "on_state_changed",
                mapOf("name" to "media", "value" to isReady.toString())
            )
        }
        Handler(Looper.getMainLooper()).post {
            MainActivity.flutterMethodChannel?.invokeMethod(
                "on_state_changed",
                mapOf("name" to "input", "value" to InputService.isOpen.toString())
            )
        }
        return isReady
    }




    private var mNotification: Notification? = null
    @SuppressLint("WrongConstant")
    private fun getNotification(text: String?, isSilent: Boolean): Notification {
        val notificationIntent = Intent(this, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this, 0,
            notificationIntent, FLAG_IMMUTABLE
        )
        val builder = NotificationCompat.Builder(
            this,
            packageName
        )
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(text?: "null text")
            .setSilent(isSilent)
            .setOngoing(true)
            .setContentIntent(pendingIntent)
        if (Build.VERSION.SDK_INT >= 31) {
            builder.setForegroundServiceBehavior(FOREGROUND_SERVICE_IMMEDIATE)
        }
        mNotification = builder.build()
        return mNotification!!
    }

    private fun updateNotification() {
        (getSystemService(NOTIFICATION_SERVICE) as NotificationManager)
            .notify(
                DEFAULT_NOTIFY_ID,
                getNotification(
                    DEFAULT_NOTIFY_TEXT,
                    false
                )
            )

    }

    fun getCurrentNotification(): Notification? {
        return try {
            instance!!.mNotification
        } catch (ignored: java.lang.Exception) {
            null
        }
    }
}
