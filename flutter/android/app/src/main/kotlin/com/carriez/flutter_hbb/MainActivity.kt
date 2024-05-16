package com.carriez.flutter_hbb

/**
 * Handle events from flutter
 * Request MediaProjection permission
 *
 * Inspired by [droidVNC-NG] https://github.com/bk138/droidVNC-NG
 */

import android.content.Context
import android.content.Intent
import android.media.MediaCodecInfo
import android.media.MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface
import android.media.MediaCodecInfo.CodecCapabilities.COLOR_FormatYUV420SemiPlanar
import android.media.MediaCodecList
import android.os.Build
import android.util.DisplayMetrics
import android.util.Log
import android.view.WindowManager
import com.hjq.permissions.XXPermissions
import ffi.FFI
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject
import kotlin.concurrent.thread


class MainActivity : FlutterActivity() {
    companion object {
        var flutterMethodChannel: MethodChannel? = null
    }

    private val channelTag = "mChannel"
    private val logTag = "mMainActivity"

    private var isAudioStart = false
    private val audioRecordHandle = AudioRecordHandle(this, { false }, { isAudioStart })

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        flutterMethodChannel = MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            channelTag
        )
        initFlutterChannel(flutterMethodChannel!!)
    }

    override fun onResume() {
        super.onResume()
    }



    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)

    }

    override fun onDestroy() {
        Log.e(logTag, "onDestroy")
        super.onDestroy()
    }



    private fun initFlutterChannel(flutterMethodChannel: MethodChannel) {
        flutterMethodChannel.setMethodCallHandler { call, result ->
            // make sure result will be invoked, otherwise flutter will await forever
            when (call.method) {
                "init_service" -> {
                    if (MainService.isReady) {
                        result.success(false)
                        return@setMethodCallHandler
                    }
                    val request = Intent(
                        this@MainActivity,
                        MainService::class.java
                    )
                    request.action = ACT_INIT_MEDIA_PROJECTION_AND_SERVICE
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                        startForegroundService(request)
                    } else {
                        startService(request)
                    }
                    result.success(true)
                }
                "start_capture" -> {

                }
                "stop_service" -> {

                }
                "check_permission" -> {
                    if (call.arguments is String) {
                        result.success(XXPermissions.isGranted(context, call.arguments as String))
                    } else {
                        result.success(false)
                    }
                }
                "request_permission" -> {

                }
                START_ACTION -> {
                    if (call.arguments is String) {
                        startAction(context, call.arguments as String)
                        result.success(true)
                    } else {
                        result.success(false)
                    }
                }
                "check_video_permission" -> {

                }
                "check_service" -> {
                    Companion.flutterMethodChannel?.invokeMethod(
                        "on_state_changed",
                        mapOf("name" to "input", "value" to InputService.isOpen.toString())
                    )
                    Companion.flutterMethodChannel?.invokeMethod(
                        "on_state_changed",
                        mapOf("name" to "media", "value" to MainService.isReady.toString())
                    )
                    result.success(true)
                }
                "stop_input" -> {

                }
                "cancel_notification" -> {
                }
                "enable_soft_keyboard" -> {

                }
                GET_START_ON_BOOT_OPT -> {

                }
                SET_START_ON_BOOT_OPT -> {

                }
                SYNC_APP_DIR_CONFIG_PATH -> {

                }
                "on_voice_call_started" -> {
                }
                "on_voice_call_closed" -> {
                }
                else -> {
                    result.error("-1", "No such method", null)
                }
            }
        }
    }

}
