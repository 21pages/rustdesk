package com.carriez.flutter_hbb

import android.annotation.SuppressLint
import android.app.PendingIntent
import android.app.Service
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
        layoutParams.x = lastLayoutX
        layoutParams.y = lastLayoutY

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager
        windowManager.addView(floatingView, layoutParams)
        moveToScreenSide(floatingView)
    }

    override fun onDestroy() {
        super.onDestroy()
        windowManager.removeView(floatingView)
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
                    moveToScreenSide(view)
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

    private fun moveToScreenSide(view: View?) {
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
        layoutParams.width = viewWidth / 2
        windowManager.updateViewLayout(view, layoutParams)
        lastLayoutX = layoutParams.x
        lastLayoutY = layoutParams.y
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        moveToScreenSide(floatingView)
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

