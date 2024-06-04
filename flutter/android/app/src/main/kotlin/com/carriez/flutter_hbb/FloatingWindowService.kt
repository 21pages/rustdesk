package com.carriez.flutter_hbb

import android.annotation.SuppressLint
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.PixelFormat
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.Drawable
import android.os.Build
import android.os.IBinder
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.view.WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
import android.view.WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
import android.view.WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.PopupMenu
import android.widget.PopupWindow
import android.widget.TextView
import kotlin.math.abs


class FloatingWindowService : Service(), View.OnTouchListener {

    private lateinit var windowManager: WindowManager
    private lateinit var layoutParams: WindowManager.LayoutParams
    private lateinit var floatingView: ImageView
    private lateinit var originalDrawable: Drawable
    private lateinit var leftHalfDrawable: Drawable
    private lateinit var rightHalfDrawable: Drawable

    private val viewWidth = 200
    private val viewHeight = 200
    private var dragging = false
    private var lastDownX = 0f
    private var lastDownY = 0f

    companion object {
        var firstLayout = true
        var lastLayoutX = 0
        var lastLayoutY = 0
    }

    override fun onBind(intent: Intent): IBinder? {
        return null
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onCreate() {
        super.onCreate()

        floatingView = ImageView(this)
        originalDrawable = resources.getDrawable(R.mipmap.ic_launcher_floating, null)
        val originalBitmap = Bitmap.createBitmap(originalDrawable.intrinsicWidth, originalDrawable.intrinsicHeight, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(originalBitmap)
        originalDrawable.setBounds(0, 0, originalDrawable.intrinsicWidth, originalDrawable.intrinsicHeight)
        originalDrawable.draw(canvas)
        val leftHalfBitmap = Bitmap.createBitmap(originalBitmap, 0, 0, originalDrawable.intrinsicWidth / 2, originalDrawable.intrinsicHeight)
        val rightHalfBitmap = Bitmap.createBitmap(originalBitmap, originalDrawable.intrinsicWidth / 2, 0, originalDrawable.intrinsicWidth / 2, originalDrawable.intrinsicHeight)
        leftHalfDrawable = BitmapDrawable(resources, leftHalfBitmap)
        rightHalfDrawable = BitmapDrawable(resources, rightHalfBitmap)

        floatingView.setImageDrawable(rightHalfDrawable)
        floatingView.setOnTouchListener(this)

        val flags =  FLAG_LAYOUT_IN_SCREEN or FLAG_NOT_TOUCH_MODAL or FLAG_NOT_FOCUSABLE
        layoutParams = WindowManager.LayoutParams(
            viewWidth / 2,
            viewHeight,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY else WindowManager.LayoutParams.TYPE_PHONE,
            flags,
            PixelFormat.TRANSLUCENT
        )

        layoutParams.gravity = Gravity.TOP or Gravity.START
        if (firstLayout) {
            firstLayout = false
            val windowManager = getSystemService(WINDOW_SERVICE) as WindowManager
            val wh = getScreenSize(windowManager)
            lastLayoutX = 0 //wh.first - viewWidth / 2
            lastLayoutY = (wh.second - viewHeight) / 2
        }
        layoutParams.x = lastLayoutX
        layoutParams.y = lastLayoutY

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager
        windowManager.addView(floatingView, layoutParams)
        moveToScreenSide(true)
    }

    override fun onDestroy() {
        super.onDestroy()
        windowManager.removeView(floatingView)
    }

    private fun performClick() {
        showPopupMenu()
//        showMenu2()
//        showMenu3()
    }

    override fun onTouch(view: View?, event: MotionEvent?): Boolean {
        when (event?.action) {
            MotionEvent.ACTION_DOWN -> {
                dragging = false
                lastDownX = event.rawX
                lastDownY = event.rawY
            }
            MotionEvent.ACTION_UP -> {
                val clickDragTolerance = 10f
                if (abs(event.rawX - lastDownX) < clickDragTolerance && abs(event.rawY - lastDownY) < clickDragTolerance) {
                    performClick()
                } else {
                    moveToScreenSide()
                }
            }
            MotionEvent.ACTION_MOVE -> {
                val dx = event.rawX - lastDownX
                val dy = event.rawY - lastDownY
                // ignore too small fist start moving(some time is click)
                if (!dragging && dx*dx+dy*dy < 25) {
                    return false
                }
                dragging = true
                layoutParams.x = event.rawX.toInt()
                layoutParams.y = event.rawY.toInt()
                layoutParams.width = viewWidth
                floatingView.setImageDrawable(originalDrawable)
                windowManager.updateViewLayout(view, layoutParams)
                lastLayoutX = layoutParams.x
                lastLayoutY = layoutParams.y
            }
        }
        return false
    }

    private fun moveToScreenSide(center: Boolean = false) {
        val windowManager = getSystemService(WINDOW_SERVICE) as WindowManager
        val wh = getScreenSize(windowManager)
        val w = wh.first
        if (layoutParams.x < w / 2) {
            layoutParams.x = 0
            floatingView.setImageDrawable(rightHalfDrawable)
        } else {
            layoutParams.x = w - viewWidth / 2
            floatingView.setImageDrawable(leftHalfDrawable)
        }
        if (center) {
            layoutParams.y = (wh.second - viewHeight) / 2
        }
        layoutParams.width = viewWidth / 2
        windowManager.updateViewLayout(floatingView, layoutParams)
        lastLayoutX = layoutParams.x
        lastLayoutY = layoutParams.y
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        moveToScreenSide(true)
    }

     private fun showPopupMenu() {
         val popupMenu = PopupMenu(this, floatingView)
         val idShowRustDesk = 0
         popupMenu.menu.add(0, idShowRustDesk, 0, translate("Show RustDesk"))
         val idStopService = 1
         popupMenu.menu.add(0, idStopService, 0, translate("Stop service"))
         popupMenu.setOnMenuItemClickListener { menuItem ->
             when (menuItem.itemId) {
                 idShowRustDesk -> {
                     openMainActivity()
                     true
                 }
                 idStopService -> {
                     stopMainService()
                     true
                 }
                 else -> false
             }
         }
         popupMenu.setOnDismissListener {
             moveToScreenSide()
         }
         popupMenu.show()
     }

//    private fun createView(context: Context): View {
//            // Create a LinearLayout
//            val layout = LinearLayout(context)
//            layout.orientation = LinearLayout.HORIZONTAL
//
//            // Create "Show RustDesk" action
//            val showRustDeskLayout = LinearLayout(context)
//            showRustDeskLayout.orientation = LinearLayout.VERTICAL
//            val showRustDeskIcon = ImageView(context)
//            // Set your icon here
//            showRustDeskIcon.setImageResource(R.mipmap.ic_launcher)
//            val showRustDeskText = TextView(context)
//            showRustDeskText.text = "Show RustDesk"
//            showRustDeskText.textSize = 12f
//            showRustDeskLayout.addView(showRustDeskIcon)
//            showRustDeskLayout.addView(showRustDeskText)
//            showRustDeskLayout.setOnClickListener {
//                openMainActivity()
//            }
//
//            // Create "Stop Service" action
//            val stopServiceLayout = LinearLayout(context)
//            stopServiceLayout.orientation = LinearLayout.VERTICAL
//            val stopServiceIcon = ImageView(context)
//            // Set your icon here
//            stopServiceIcon.setImageResource(R.drawable.close_red)
//            val stopServiceText = TextView(context)
//            stopServiceText.text = "Stop Service"
//            stopServiceText.textSize = 12f
//            stopServiceLayout.addView(stopServiceIcon)
//            stopServiceLayout.addView(stopServiceText)
//            stopServiceLayout.setOnClickListener {
//                stopMainService()
//            }
//
//            // Add actions to LinearLayout
//            layout.addView(showRustDeskLayout)
//            layout.addView(stopServiceLayout)
//
//        return layout
//    }
//
//    private fun showMenu2() {
//        val popupWindow = PopupWindow(this)
//
//        popupWindow.contentView = createView(this)
//        popupWindow.width = LinearLayout.LayoutParams.WRAP_CONTENT
//        popupWindow.height = LinearLayout.LayoutParams.WRAP_CONTENT
//        popupWindow.isFocusable = true
//        popupWindow.showAsDropDown(floatingView, 0, 0)
//    }
//
//
//    private fun showMenu3() {
//        // 创建一个 PopupWindow
//        val popupWindow = PopupWindow(this)
//
//        // 创建一个 LinearLayout 作为 PopupWindow 的内容
//        val layout = LinearLayout(this)
//        layout.orientation = LinearLayout.VERTICAL
//
//        // 创建菜单项
//        val menuItem1 = TextView(this)
//        menuItem1.text = "Show RustDesk"
//        menuItem1.setOnClickListener {
//            openMainActivity()
//            popupWindow.dismiss()
//        }
//        val menuItem2 = TextView(this)
//        menuItem2.text = "Stop service"
//        menuItem2.setOnClickListener {
//            stopMainService()
//            popupWindow.dismiss()
//        }
//
//        // 将菜单项添加到 LinearLayout
//        layout.addView(menuItem1)
//        layout.addView(menuItem2)
//
//        // 为 LinearLayout 设置边距
//        val params = LinearLayout.LayoutParams(
//            LinearLayout.LayoutParams.WRAP_CONTENT,
//            LinearLayout.LayoutParams.WRAP_CONTENT
//        )
//        val marginInPixels = 50 // 设置为所需的边距值
//        params.setMargins(marginInPixels, marginInPixels, marginInPixels, marginInPixels)
//        layout.layoutParams = params
//
//        // 将 LinearLayout 设置为 PopupWindow 的内容
//        popupWindow.contentView = layout
//
//        // 设置 PopupWindow 的宽度和高度
//        popupWindow.width = LinearLayout.LayoutParams.WRAP_CONTENT
//        popupWindow.height = LinearLayout.LayoutParams.WRAP_CONTENT
//
//        // 显示 PopupWindow
//        popupWindow.showAsDropDown(floatingView)
//    }

    private fun openMainActivity() {
        val intent = Intent(this, MainActivity::class.java)
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_ONE_SHOT
        )
        try {
            pendingIntent.send()
        } catch (e: PendingIntent.CanceledException) {
            e.printStackTrace()
        }
    }

    private fun stopMainService() {
        MainActivity.flutterMethodChannel?.invokeMethod("stop_service", null)
    }
}

