package com.carriez.flutter_hbb

import android.annotation.SuppressLint
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Bitmap
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
import android.view.Display
import android.view.WindowManager
import ffi.FFI
import java.nio.ByteBuffer
import kotlin.math.max
import kotlin.math.min


class MediaProjectionService : Service() {
    private var mResultCode = 0
    private var mResultData: Intent? = null
    private var mImageReader: ImageReader? = null
    private var mVirtualDisplay: VirtualDisplay? = null
    private var mMediaProjection: MediaProjection? = null
    private var mMediaProjectionManager: MediaProjectionManager? = null
    private var mHasPortraitInLandscapeWorkaroundApplied = false
    private var mHasPortraitInLandscapeWorkaroundSet = false
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
         */mMediaProjectionManager =
            getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        updateScreenInfo(newConfig.orientation)
//        val displayMetrics: DisplayMetrics = Utils.getDisplayMetrics(this, Display.DEFAULT_DISPLAY)
//        Log.d(
//            TAG,
//            "onConfigurationChanged: width: " + displayMetrics.widthPixels + " height: " + displayMetrics.heightPixels
//        )
//        startScreenCapture()
    }

    override fun onDestroy() {
        Log.d(TAG, "onDestroy")
        stopScreenCapture()
        instance = null
    }

    override fun onStartCommand(intent: Intent, flags: Int, startId: Int): Int {
        mResultCode = intent.getIntExtra(EXTRA_MEDIA_PROJECTION_RESULT_CODE, 0)
        mResultData =
            intent.getParcelableExtra<Intent>(EXTRA_MEDIA_PROJECTION_RESULT_DATA)
        startScreenCapture()

//        return START_REDELIVER_INTENT;
        return START_NOT_STICKY
    }

    @SuppressLint("WrongConstant")
    private fun startScreenCapture() {
        updateScreenInfo(resources.configuration.orientation)
        if (mMediaProjection == null) mMediaProjection = try {
            mMediaProjectionManager!!.getMediaProjection(
                mResultCode,
                mResultData!!
            )
        } catch (e: SecurityException) {
            Log.w(TAG, "startScreenCapture: got SecurityException, re-requesting confirmation")
            // This initiates a prompt dialog for the user to confirm screen projection.
            val mediaProjectionRequestIntent = Intent(
                this,
                MediaProjectionRequestActivity::class.java
            )
            mediaProjectionRequestIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            startActivity(mediaProjectionRequestIntent)
            return
        }
        if (mMediaProjection == null) {
            Log.e(TAG, "startScreenCapture: did not get a media projection, probably user denied")
            return
        }

        // Android 14 and newer require this callback
        mMediaProjection!!.registerCallback(
            object : MediaProjection.Callback() {
                override fun onStop() {
                    Log.d(TAG, "callback: onStop")
                    super.onStop()
                }

                fun onCapturedContentResize(width: Int, height: Int) {
                    Log.d(TAG, "callback: onCapturedContentResize " + width + "x" + height)
                }

                fun onCapturedContentVisibilityChanged(isVisible: Boolean) {
                    Log.d(
                        TAG,
                        "callback: onCapturedContentVisibilityChanged $isVisible"
                    )
                }
            },
            null
        )
        if (mImageReader != null) mImageReader!!.close()
        val metrics: DisplayMetrics = DisplayMetrics()
        val windowManager = getSystemService(Context.WINDOW_SERVICE) as WindowManager
        windowManager.defaultDisplay.getRealMetrics(metrics)

        // apply selected scaling
        val scaling: Float = 1.0f
        val scaledWidth = (metrics.widthPixels * scaling).toInt()
        val scaledHeight = (metrics.heightPixels * scaling).toInt()

        // only set this by detecting quirky hardware if the user has not set manually
        if (!mHasPortraitInLandscapeWorkaroundSet && Build.FINGERPRINT.contains("rk3288") && metrics.widthPixels > 800) {
            Log.w(TAG, "detected >10in rk3288 applying workaround for portrait-in-landscape quirk")
            mHasPortraitInLandscapeWorkaroundApplied = true
        }


        /*
            This is the default behaviour.
         */mImageReader =
            ImageReader.newInstance(scaledWidth, scaledHeight, PixelFormat.RGBA_8888, 2)
        mImageReader!!.setOnImageAvailableListener({ imageReader: ImageReader ->
            try {
                imageReader.acquireLatestImage().use { image ->
                    if (image == null) return@setOnImageAvailableListener
                    val planes = image.planes
                    val buffer = planes[0].buffer
                    FFI.onVideoFrameUpdate(buffer)

                }
            } catch (ignored: Exception) {
            }
        }, null)
        try {
            if (mVirtualDisplay == null) {
                mVirtualDisplay = mMediaProjection!!.createVirtualDisplay(
                    getString(R.string.app_name),
                    scaledWidth, scaledHeight, metrics.densityDpi,
                    DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                    mImageReader!!.surface, null, null
                )
            } else {
                mVirtualDisplay!!.resize(scaledWidth, scaledHeight, metrics.densityDpi)
                mVirtualDisplay!!.surface = mImageReader!!.surface
            }
        } catch (e: SecurityException) {
            Log.w(TAG, "startScreenCapture: got SecurityException, re-requesting confirmation")
            // This initiates a prompt dialog for the user to confirm screen projection.
            val mediaProjectionRequestIntent = Intent(
                this,
                MediaProjectionRequestActivity::class.java
            )
            mediaProjectionRequestIntent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            startActivity(mediaProjectionRequestIntent)
        }
    }

    private fun stopScreenCapture() {
        try {
            mVirtualDisplay!!.release()
            mVirtualDisplay = null
        } catch (e: Exception) {
            //unused
        }
        if (mMediaProjection != null) {
            mMediaProjection!!.stop()
            mMediaProjection = null
        }
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
            if (w > 1200 || h > 1200) {
                scale = 2
                w /= scale
                h /= scale
                dpi /= scale
            }
            if (SCREEN_INFO.width != w) {
                SCREEN_INFO.width = w
                SCREEN_INFO.height = h
                SCREEN_INFO.scale = scale
                SCREEN_INFO.dpi = dpi
                if (MainService.isStart) {
                    stopScreenCapture()
                    FFI.refreshScreen()
                    startScreenCapture()
                }
            }

        }
    }

    companion object {
        private const val TAG = "MediaProjectionService"
        private var instance: MediaProjectionService? = null
        val isMediaProjectionEnabled: Boolean
            /**
             * Get whether Media Projection was granted by the user.
             */
            get() = instance != null && instance!!.mResultCode != 0 && instance!!.mResultData != null

        fun togglePortraitInLandscapeWorkaround() {
            try {
                // set
                instance!!.mHasPortraitInLandscapeWorkaroundSet = true
                instance!!.mHasPortraitInLandscapeWorkaroundApplied =
                    !instance!!.mHasPortraitInLandscapeWorkaroundApplied
                Log.d(
                    TAG,
                    "togglePortraitInLandscapeWorkaround: now " + instance!!.mHasPortraitInLandscapeWorkaroundApplied
                )
                // apply
                instance!!.startScreenCapture()
            } catch (e: NullPointerException) {
                //unused
            }
        }
    }
}


