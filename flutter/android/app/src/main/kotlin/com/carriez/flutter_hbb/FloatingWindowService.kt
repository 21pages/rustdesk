package com.carriez.flutter_hbb

import android.app.Service
import android.content.Intent
import android.graphics.Color
import android.graphics.PixelFormat
import android.os.Build
import android.os.IBinder
import android.view.Gravity
import android.view.WindowManager
import android.widget.TextView

class FloatingWindowService : Service() {

    private lateinit var windowManager: WindowManager
    private lateinit var params: WindowManager.LayoutParams
    private lateinit var floatingView: TextView

    override fun onBind(intent: Intent): IBinder? {
        return null
    }

    override fun onCreate() {
        super.onCreate()

        // 创建悬浮窗口
        floatingView = TextView(this)
        floatingView.text = "Floating Window"
        floatingView.setBackgroundColor(Color.BLUE)

        // 设置悬浮窗口的布局参数
        params = WindowManager.LayoutParams(
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.WRAP_CONTENT,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY else WindowManager.LayoutParams.TYPE_PHONE,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE,
            PixelFormat.TRANSLUCENT
        )

        // 设置悬浮窗口的位置
        params.gravity = Gravity.TOP or Gravity.START
        params.x = 0
        params.y = 100

        // 获取WindowManager服务
        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager

        // 将悬浮窗口添加到屏幕上
        windowManager.addView(floatingView, params)
    }

    override fun onDestroy() {
        super.onDestroy()
        // 移除悬浮窗口
        windowManager.removeView(floatingView)
    }
}
