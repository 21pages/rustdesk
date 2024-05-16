package com.carriez.flutter_hbb

import android.annotation.SuppressLint
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
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
//                if (Build.VERSION.SDK_INT >= 29) {
//                    // throws NullPointerException if no notification
//                    startForeground(MainService.NOTIFICATION_ID, Objects.requireNonNull(MainService.getCurrentNotification()), ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION);
//                } else {
                // throws IllegalArgumentException if no notification
                startForeground(DEFAULT_NOTIFY_ID, MainService.instance?.getCurrentNotification())
                //                }
            } catch (ignored: Exception) {
                Log.e(TAG, "Not starting because MainService quit")
            }
        }

        /*
            Get the MediaProjectionManager
         */
        mMediaProjectionManager =
            getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        updateScreenInfo(newConfig.orientation)
    }

    override fun onDestroy() {
        Log.d(TAG, "onDestroy")
        stopCapture()
        instance = null
        stopCapture()

        if (reuseVirtualDisplay) {
            virtualDisplay?.release()
            virtualDisplay = null
        }
        mediaProjection = null
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

    @SuppressLint("WrongConstant")
    private fun createSurface(): Surface? {

        Log.d(logTag, "ImageReader.newInstance:INFO:$SCREEN_INFO")
        imageReader =
            ImageReader.newInstance(
                SCREEN_INFO.width,
                SCREEN_INFO.height,
                PixelFormat.RGBA_8888,
                4
            ).apply {
                setOnImageAvailableListener({ imageReader: ImageReader ->
                    try {
                        // If not call acquireLatestImage, listener will not be called again
                        imageReader.acquireLatestImage().use { image ->
                            if (image == null || !MainService.isStart) return@setOnImageAvailableListener
                            val planes = image.planes
                            val buffer = planes[0].buffer
                            buffer.rewind()
                           FFI.onVideoFrameUpdate(buffer)
                        }
                    } catch (ignored: java.lang.Exception) {
                    }
                }, null)
            }
        Log.d(logTag, "ImageReader.setOnImageAvailableListener done")
        return imageReader?.surface
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
//            if (w > 1200 || h > 1200) {
//                scale = 2
//                w /= scale
//                h /= scale
//                dpi /= scale
//            }
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
        if (MainService.isStart) {
            return true
        }
        if (mediaProjection == null) try {
            mediaProjection =
                mMediaProjectionManager!!.getMediaProjection(mResultCode, mResultData!!)
        } catch (e: SecurityException) {
            Log.w(TAG, "startScreenCapture: got SecurityException, re-requesting confirmation")
            // This initiates a prompt dialog for the user to confirm screen projection.
            val mediaProjectionRequestIntent = Intent(
                this,
                MediaProjectionRequestActivity::class.java
            )
            mediaProjectionRequestIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            startActivity(mediaProjectionRequestIntent)
            return false
        }
        if (mediaProjection == null) {
            Log.w(logTag, "startCapture fail,mediaProjection is null")
            return false
        }

        Log.d(logTag, "Start Capture")
        surface = createSurface()
        startRawVideoRecorder(mediaProjection!!)

        MainService._isStart = true
        FFI.setFrameRawEnable("video",true)
        // if (wakeLock.isHeld) {
        //     wakeLock.release()
        // }
        // wakeLock.acquire()
        return true
    }

    @Synchronized
    fun stopCapture() {
        Log.d(logTag, "Stop Capture")
        FFI.setFrameRawEnable("video",false)
        MainService.setIsStart(false)
        // release video
        if (reuseVirtualDisplay) {
            // The virtual display video projection can be paused by calling `setSurface(null)`.
            // https://developer.android.com/reference/android/hardware/display/VirtualDisplay.Callback
            // https://learn.microsoft.com/en-us/dotnet/api/android.hardware.display.virtualdisplay.callback.onpaused?view=net-android-34.0
            virtualDisplay?.setSurface(null)
        } else {
            virtualDisplay?.release()
        }
        // suface needs to be release after `imageReader.close()` to imageReader access released surface
        // https://github.com/rustdesk/rustdesk/issues/4118#issuecomment-1515666629
        imageReader?.close()
        imageReader = null
        if (!reuseVirtualDisplay) {
            virtualDisplay = null
        }
        // suface needs to be release after `imageReader.close()` to imageReader access released surface
        // https://github.com/rustdesk/rustdesk/issues/4118#issuecomment-1515666629
        surface?.release()

        // release audio
        MainService._isAudioStart = false
    }

    private fun startRawVideoRecorder(mp: MediaProjection) {
        Log.d(logTag, "startRawVideoRecorder,screen info:$SCREEN_INFO")
        if (surface == null) {
            Log.d(logTag, "startRawVideoRecorder failed,surface is null")
            return
        }
        createOrSetVirtualDisplay(mp, surface!!)
    }

    // https://github.com/bk138/droidVNC-NG/blob/b79af62db5a1c08ed94e6a91464859ffed6f4e97/app/src/main/java/net/christianbeier/droidvnc_ng/MediaProjectionService.java#L250
    // Reuse virtualDisplay if it exists, to avoid media projection confirmation dialog every connection.
    private fun createOrSetVirtualDisplay(mp: MediaProjection, s: Surface) {
        try {
            virtualDisplay?.let {
                it.resize(SCREEN_INFO.width, SCREEN_INFO.height, SCREEN_INFO.dpi)
                it.setSurface(s)
            } ?: let {
                virtualDisplay = mp.createVirtualDisplay(
                    "RustDeskVD",
                    SCREEN_INFO.width, SCREEN_INFO.height, SCREEN_INFO.dpi,
                    DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                    s, null, null
                )
            }
        } catch (e: SecurityException) {
            Log.w(logTag, "createOrSetVirtualDisplay: got SecurityException, re-requesting confirmation");
            // This initiates a prompt dialog for the user to confirm screen projection.
        }
    }

    companion object {
        private const val TAG = "MediaProjectionService"
        public var instance: MediaProjectionService? = null
    }
}


