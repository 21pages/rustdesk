package com.carriez.flutter_hbb

import android.annotation.SuppressLint
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.content.res.Configuration
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.ImageReader
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.IBinder
import android.util.DisplayMetrics
import android.util.Log
import android.view.Surface
import android.view.WindowManager
import ffi.FFI
import java.util.Objects
import kotlin.math.max
import kotlin.math.min


class MediaProjectionService : Service() {
    private var mResultCode = 0
    private var mResultData: Intent? = null
    private var imageReader: ImageReader? = null
    private var virtualDisplay: VirtualDisplay? = null
    private var mediaProjection: MediaProjection? = null
    private var surface: Surface? = null
    private var mMediaProjectionManager: MediaProjectionManager? = null
    private var reuseVirtualDisplay = Build.VERSION.SDK_INT > 33
    private val logTag = "MediaProjectionService"

    override fun onBind(intent: Intent): IBinder? {
        return null
    }

    override fun onCreate() {
        Log.d(TAG, "onCreate")
        instance = this
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

            /*
                startForeground() w/ notification; bit hacky re-using MainService's ;-)
             */try {
                if (Build.VERSION.SDK_INT >= 29) {
                    // throws NullPointerException if no notification
                    Objects.requireNonNull(MainService.instance?.getCurrentNotification())?.let {
                        startForeground(DEFAULT_NOTIFY_ID, it, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION)
                    }
                } else {
                // throws IllegalArgumentException if no notification
                startForeground(DEFAULT_NOTIFY_ID, MainService.instance?.getCurrentNotification())
                }
            } catch (ignored: Exception) {
                Log.e(TAG, "Not starting because MainService quit")
            }
        }

    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        updateScreenInfo(newConfig.orientation)
    }

    override fun onDestroy() {
        stopCapture()
        instance = null
    }

    override fun onStartCommand(intent: Intent, flags: Int, startId: Int): Int {
        mResultCode = intent.getIntExtra(EXTRA_MEDIA_PROJECTION_RESULT_CODE, 0)
        mResultData =
            intent.getParcelableExtra<Intent>(EXTRA_MEDIA_PROJECTION_RESULT_DATA)
        updateScreenInfo(resources.configuration.orientation)
        startCapture()

//        return START_REDELIVER_INTENT;
        return START_NOT_STICKY
    }

    private fun updateScreenInfo(orientation: Int) {
        var w: Int
        var h: Int
        var dpi: Int
        val windowManager = getSystemService(Context.WINDOW_SERVICE) as WindowManager

        @Suppress("DEPRECATION")
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val m = windowManager.maximumWindowMetrics
            w = m.bounds.width()
            h = m.bounds.height()
            dpi = resources.configuration.densityDpi
        } else {
            val dm = DisplayMetrics()
            windowManager.defaultDisplay.getRealMetrics(dm)
            w = dm.widthPixels
            h = dm.heightPixels
            dpi = dm.densityDpi
        }

        val max = max(w,h)
        val min = min(w,h)
        if (orientation == Configuration.ORIENTATION_LANDSCAPE) {
            w = max
            h = min
        } else {
            w = min
            h = max
        }
        var scale = 1
        if (w != 0 && h != 0) {
            if (SCREEN_INFO.width != w) {
                SCREEN_INFO.width = w
                SCREEN_INFO.height = h
                SCREEN_INFO.scale = scale
                SCREEN_INFO.dpi = dpi
                if (MainService.isStart) {
                    startCapture()
                    FFI.refreshScreen()
                    stopCapture()
                }
            }

        }
    }

    fun startCapture(): Boolean {
        MainService._isStart = true
        return true
    }

    @Synchronized
    fun stopCapture() {
        MainService.setIsStart(false)
    }

    private fun startRawVideoRecorder(mp: MediaProjection) {
    }

    // https://github.com/bk138/droidVNC-NG/blob/b79af62db5a1c08ed94e6a91464859ffed6f4e97/app/src/main/java/net/christianbeier/droidvnc_ng/MediaProjectionService.java#L250
    // Reuse virtualDisplay if it exists, to avoid media projection confirmation dialog every connection.
    private fun createOrSetVirtualDisplay(mp: MediaProjection, s: Surface) {
    }

    companion object {
        private const val TAG = "MediaProjectionService"
        public var instance: MediaProjectionService? = null
    }
}


