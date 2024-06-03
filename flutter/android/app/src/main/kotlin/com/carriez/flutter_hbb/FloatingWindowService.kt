package com.carriez.flutter_hbb

import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.graphics.PixelFormat
import android.graphics.drawable.Drawable
import android.os.Build
import android.os.IBinder
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.view.WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
import android.view.WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
import android.view.WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL
import android.widget.ImageView
import kotlin.math.abs


class FloatingWindowService : Service(), View.OnTouchListener {

    private lateinit var windowManager: WindowManager
    private lateinit var layoutParams: WindowManager.LayoutParams
    private lateinit var floatingView: ImageView

    private var dragging = false
    private var lastDownX = 0f
    private var lastDownY = 0f
    private val CLICK_DRAG_TOLERANCE = 10f 

    override fun onBind(intent: Intent): IBinder? {
        return null
    }

    override fun onCreate() {
        super.onCreate()

        floatingView = ImageView(this)

        val drawable: Drawable = resources.getDrawable(R.mipmap.ic_launcher_floating, null)
        // drawable.alpha = 255
        floatingView.setImageDrawable(drawable)


        floatingView.setOnTouchListener(this)

        val flags = FLAG_LAYOUT_IN_SCREEN or FLAG_NOT_TOUCH_MODAL or FLAG_NOT_FOCUSABLE
        layoutParams = WindowManager.LayoutParams(
            100,
            100,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY else WindowManager.LayoutParams.TYPE_PHONE,
            flags,
            PixelFormat.TRANSLUCENT
        )

        layoutParams.gravity = Gravity.TOP or Gravity.START
        layoutParams.x = lastDownX.toInt()
        layoutParams.y = lastDownY.toInt()

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager

        windowManager.addView(floatingView, layoutParams)
    }

    private fun performClick() {
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

    override fun onDestroy() {
        super.onDestroy()
        windowManager.removeView(floatingView)
    }

    override fun onTouch(view: View?, event: MotionEvent?): Boolean {
        when (event?.action) {
            MotionEvent.ACTION_DOWN -> {
                dragging = false
                lastDownX = event.rawX
                lastDownY = event.rawY
            }
            MotionEvent.ACTION_UP -> {
                if (abs(event.rawX - lastDownX) < CLICK_DRAG_TOLERANCE && abs(event.rawY - lastDownY) < CLICK_DRAG_TOLERANCE) {
                    performClick()
                } else {
                    val windowManager = getSystemService(Context.WINDOW_SERVICE) as WindowManager
                    val wh = getScreenSize(windowManager)
                    var w = wh.first
                    var h = wh.second
                    if (layoutParams.x < w / 2) {
                        layoutParams.x = 0
                    } else {
                        layoutParams.x = w
                    }
                    windowManager.updateViewLayout(view, layoutParams)
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
                windowManager.updateViewLayout(view, layoutParams)
            }
        }
        return false
    }

    // private fun showPopupMenu(view: View) {
    //     // Create a PopupMenu, assign it a Menu XML file, and show it
    //     val popupMenu = PopupMenu(this, view)
    //     popupMenu.menu.add(0, 0, 0, "close connection")

    //     popupMenu.setOnMenuItemClickListener { menuItem ->
    //         when (menuItem.itemId) {
    //             0 -> {
    //                 // Handle the "Close Connection" menu item click here
    //                 true
    //             }
    //             else -> false
    //         }
    //     }
    //     popupMenu.show()
    // }
}

